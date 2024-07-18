use std::{ops::Sub, sync::Arc};

use crate::utils;
use apache_avro::AvroSchema;
use opentelemetry::{
    global,
    trace::{Span, TraceContextExt, Tracer},
    Context, Key, KeyValue, StringValue,
};
use rdkafka::{
    message::{Header, OwnedHeaders},
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use schema_registry_converter::{
    async_impl::{easy_avro::EasyAvroEncoder, schema_registry::SrSettings},
    avro_common::get_supplied_schema,
    schema_registry_common::SubjectNameStrategy,
};
use serde::Serialize;
use std::time::Duration;
use tracing::{error, info};

pub struct KafkaProducer {
    producer: FutureProducer,
    avro_encoder: Arc<EasyAvroEncoder>,
    topic: String,
}

impl KafkaProducer {
    pub fn new(bootstrap_servers: String, schema_registry_url: String, topic: String) -> Self {
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap_servers)
            .set("produce.offset.report", "true")
            .set("message.timeout.ms", "5000")
            .set("queue.buffering.max.messages", "10")
            .create()
            .expect("Unable to create producer");

        let sr_settings = SrSettings::new(schema_registry_url);
        let avro_encoder = EasyAvroEncoder::new(sr_settings);
        Self {
            avro_encoder: Arc::new(avro_encoder),
            producer,
            topic,
        }
    }
    pub async fn produce<T: Serialize + AvroSchema>(&self, key: String, payload: T) -> bool {
        let value_strategy = SubjectNameStrategy::TopicNameStrategyWithSchema(
            self.topic.clone(),
            true,
            get_supplied_schema(&T::get_schema()),
        );
        let payload = match self
            .avro_encoder
            .clone()
            .encode_struct(payload, &value_strategy)
            .await
        {
            Ok(v) => v,
            Err(e) => panic!("Error gettign payload: {}", e),
        };

        let mut span = global::tracer("producer").start("produce_to_kafka");
        span.set_attribute(KeyValue {
            key: Key::new("topic"),
            value: opentelemetry::Value::String(StringValue::from(self.topic.clone())),
        });
        let context = Context::current_with_span(span);
        let mut headers = OwnedHeaders::new().insert(Header {
            key: "key",
            value: Some("value"),
        });
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut utils::HeaderInjector(&mut headers))
        });
        let record = FutureRecord::to(&self.topic)
            .payload(&payload)
            .key(&key)
            .headers(headers);

        let delivery_status = self.producer.send(record, Duration::from_secs(5)).await;
        if delivery_status.is_err() {
            error!("{}", delivery_status.err().unwrap().0.to_string());
            return false;
        } else {
            info!("message delivered");
            return true;
        }
    }
}
