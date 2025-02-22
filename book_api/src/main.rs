pub mod dto;
pub mod entity;
pub mod http_servers;
pub mod repository;
pub mod service;

use apache_avro::AvroSchema;
use common::events::dto::CreatedBook;
use database::get_connection;
use http_servers::start_http_server;
use kafka::utils::register_schema;
use opentelemetry::global;
use repository::Repository;
use service::{book_created_producer::BookCreatedProducer, Service};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    opentelemetry::global::set_text_map_propagator(opentelemetry_zipkin::Propagator::new());
    let tracer = opentelemetry_zipkin::new_pipeline()
        .with_service_name("books_api".to_owned())
        .with_service_address("127.0.0.1:8090".parse().to_owned().unwrap())
        .with_collector_endpoint("http://localhost:9411/api/v2/spans")
        .install_batch(opentelemetry::runtime::Tokio)
        .expect("unable to install zipkin tracer");
    let t = tracing_opentelemetry::layer().with_tracer(tracer.clone());

    let subscriber = tracing_subscriber::fmt::layer().json();

    let level = EnvFilter::new("debug".to_owned());

    tracing_subscriber::registry()
        .with(subscriber)
        .with(level)
        .with(t)
        .init();

    let db_conn =
        get_connection("postgres://postgres:postgres@localhost:5433/rust-superapp").await?;
    let repository = Repository::new(db_conn.clone())
        .await
        .expect("Error creating repository");

    let schema_registry_url = "http://localhost:8081".to_owned();
    let book_created_producer =
        BookCreatedProducer::new("localhost:9092".to_owned(), schema_registry_url.clone());
    let service = Service::new(repository, book_created_producer);

    register_schema(
        schema_registry_url,
        "book".to_string(),
        CreatedBook::get_schema(),
    )
    .await
    .expect("Error while registering schema");

    start_http_server(service).await;
    global::shutdown_tracer_provider();
    Ok(())
}
