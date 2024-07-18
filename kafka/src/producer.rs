use std::sync::Arc;

use rdkafka::{producer::FutureProducer, ClientConfig};
use schema_registry_converter::async_impl::{
    easy_avro::EasyAvroEncoder, schema_registry::SrSettings,
};

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
}
