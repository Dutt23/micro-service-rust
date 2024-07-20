use std::{env, ops::Sub, sync::Arc};

use crate::utils;
use apache_avro::AvroSchema;
use dotenv::dotenv;
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
use reqwest::{header, Client};
use schema_registry_converter::{
    async_impl::{
        easy_avro::EasyAvroEncoder,
        schema_registry::{SrSettings, SrSettingsBuilder},
    },
    avro_common::get_supplied_schema,
    schema_registry_common::SubjectNameStrategy,
};
use serde::Serialize;
use std::time::Duration;
use tracing::{error, info};
pub struct KafkaProducer {
    pub producer: FutureProducer,
    avro_encoder: Arc<EasyAvroEncoder>,
}

impl KafkaProducer {
    pub fn new(bootstrap_servers: String, schema_registry_url: String) -> Self {
        dotenv().ok();
        let api_key = env::var("KAFKA_API_KEY").expect("kafka key not found in variables");
        let api_password =
            env::var("KAFKA_API_SECRET").expect("kafka secrets not found in variables");

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", bootstrap_servers)
            .set("produce.offset.report", "true")
            .set("message.timeout.ms", "45000")
            .set("queue.buffering.max.messages", "10")
            .set("sasl.username", api_key)
            .set("sasl.password", api_password)
            .set("sasl.mechanisms", "PLAIN")
            .set("security.protocol", "SASL_SSL")
            .create()
            .expect("Unable to create producer");
        let schema_api_key =
            env::var("SCHEMA_API_KEY").expect("schema key key not found in variables");
        let schema_api_password =
            env::var("SCHEMA_API_SECRET").expect("schema password key not found in variables");
        let sr_settings = SrSettings::new_builder(schema_registry_url)
            .set_basic_authorization(&schema_api_key, Some(&schema_api_password))
            // .build_with(builder)
            .build()
            .expect("msg");
        // let sr_settings = SrSettings::new(schema_registry_url);
        let avro_encoder = EasyAvroEncoder::new(sr_settings);
        // dbg!(&avro_encoder);
        Self {
            avro_encoder: Arc::new(avro_encoder),
            producer,
        }
    }
    pub async fn produce<T: Serialize + AvroSchema>(
        &self,
        key: String,
        msg: T,
        topic: String,
    ) -> bool {
        let value_strategy = SubjectNameStrategy::TopicNameStrategy(topic.clone(), false);
        let payload = match self
            .avro_encoder
            .clone()
            .encode_struct(msg, &value_strategy)
            .await
        {
            Ok(v) => v,
            Err(e) => panic!("Error getting payload: {}", e),
        };

        let mut span = global::tracer("producer").start("produce_to_kafka");
        span.set_attribute(KeyValue {
            key: Key::new(topic.clone()),
            value: opentelemetry::Value::String(StringValue::from(topic.clone())),
        });
        let context = Context::current_with_span(span);
        let mut headers = OwnedHeaders::new().insert(Header {
            key: &key,
            value: Some("value"),
        });

        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&context, &mut utils::HeaderInjector(&mut headers))
        });
        let record = FutureRecord::to(&topic)
            .payload(&payload)
            .key(&key)
            .headers(headers);

        let delivery_status = self.producer.send(record, Duration::from_secs(60)).await;
        if delivery_status.is_err() {
            error!("{}", delivery_status.err().unwrap().0.to_string());
            return false;
        } else {
            info!("message delivered");
            return true;
        }
    }
}
