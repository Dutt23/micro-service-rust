use std::net::SocketAddr;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{post, Route},
    Extension, Json, Router,
};
use axum_tracing_opentelemetry::opentelemetry_tracing_layer;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::service::{Service, ServiceError};
use tracing::{error, info};

pub async fn start_http_server(service: Service) {
    let books_router = Router::new().route("/", post(create_book));
    let api_router = Router::new().nest("/books", books_router);
    let app = Router::new()
        .nest("/api", api_router)
        .layer(opentelemetry_tracing_layer())
        .layer(Extension(service));
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap()
}

impl IntoResponse for ServiceError {
    fn into_response(self) -> axum::response::Response {
        error!("Service Error {}", self);
        let (status, error_msg) = match self {
            ServiceError::RepositoryError(re) => {
                (StatusCode::INTERNAL_SERVER_ERROR, re.to_string())
            }
            e => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        let body = Json(json!({ "error" : error_msg}));
        (status, body).into_response()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateBookRequest {
    title: String,
    isbn: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CreateBookResponse {
    id: i32,
    title: String,
    isbn: String,
}

async fn create_book(
    Extension(service): Extension<Service>,
    Json(create_book_req): Json<CreateBookResponse>,
) -> impl IntoResponse {
    let created_book_res = service
        .create_book(create_book_req.title, create_book_req.isbn)
        .await;
    match created_book_res {
        Ok(book) => (
            StatusCode::CREATED,
            Json(json!(CreateBookResponse {
                id: book.id,
                title: book.title,
                isbn: book.isbn,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string()})),
        ),
    }
}
