use crate::{commons::create_schema_registry_settings, utils::HeaderExtractor};
use apache_avro::from_value;
use dotenv::dotenv;
use opentelemetry::{
    global,
    trace::{Span, Tracer},
    Context,
};
use rdkafka::{
    config::RDKafkaLogLevel,
    consumer::{CommitMode, Consumer, StreamConsumer},
    ClientConfig, Message,
};
use schema_registry_converter::async_impl::{
    easy_avro::EasyAvroDecoder, schema_registry::SrSettings,
};
use serde::Deserialize;
use std::{env, fmt::Debug};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info};

pub struct KafkaConsumer {
    consumer: StreamConsumer,
    avro_decoder: EasyAvroDecoder,
    topic: String,
}

impl KafkaConsumer {
    pub fn new(
        bootstrap_servers: String,
        group_id: String,
        topic: String,
        schema_registry_url: String,
    ) -> Self {
        dotenv().ok();
        // let api_key = env::var("KAFKA_API_KEY").expect("kafka key not found in variables");
        // let api_password =
        //     env::var("KAFKA_API_SECRET").expect("kafka secrets not found in variables");
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", bootstrap_servers)
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("allow.auto.create.topics", "true")
            // .set("sasl.username", api_key)
            // .set("sasl.password", api_password)
            .set_log_level(RDKafkaLogLevel::Debug)
            .create()
            .expect("Consumer creation error");
        let sr_settings = create_schema_registry_settings(schema_registry_url);
        let avro_decoder = EasyAvroDecoder::new(sr_settings);
        Self {
            consumer,
            topic,
            avro_decoder,
        }
    }

    pub async fn consume<T: Clone + Debug + for<'a> Deserialize<'a>>(
        &self,
        sender: UnboundedSender<T>,
    ) {
        self.consumer
            .subscribe(&[&self.topic])
            .expect("Can't subscribe to topics");

        while let Ok(msg) = self.consumer.recv().await {
            let context = if let Some(headers) = msg.headers() {
                global::get_text_map_propagator(|propagator| {
                    propagator.extract(&HeaderExtractor(&headers))
                })
            } else {
                Context::current()
            };

            let mut span =
                global::tracer("consumer").start_with_context("consume_payload", &context);

            let value_result = match self.avro_decoder.decode(msg.payload()).await {
                Ok(r) => Ok(r),
                Err(e) => {
                    error!("Error getting value {}", e);
                    Err(e)
                }
            };

            if let Ok(value) = value_result {
                if let Ok(deserialized_payload) = from_value::<T>(&value.value) {
                    info!(  "key: '{:?}', payload: '{:?}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                                    msg.key(),
                                    deserialized_payload,
                                    msg.topic(),
                                    msg.partition(),
                                    msg.offset(),
                                    msg.timestamp());
                    if let Err(e) = sender.send(deserialized_payload) {
                        error!("Error while sending via channel: {}", e);
                    } else {
                        info!("Message consumed successfully");
                    }
                } else {
                    error!("Error while deserializing message payload");
                }
            } else {
                error!("Error while deserializing message payload");
            }
            self.consumer
                .commit_message(&msg, CommitMode::Async)
                .unwrap();
            span.end();
        }
    }
}
