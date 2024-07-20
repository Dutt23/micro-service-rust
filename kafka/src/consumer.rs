use rdkafka::consumer::StreamConsumer;

pub struct KafkaConsumer {
    consumer: StreamConsumer,
}
