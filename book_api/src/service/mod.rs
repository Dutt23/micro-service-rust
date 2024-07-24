use std::sync::Arc;

use book_created_producer::{BookCreatedProducer, BookCreatedProducerError};
use thiserror::Error;
use tracing::{info_span, Instrument};

use crate::{
    dto::{Book, BookBuilder, BookBuilderError},
    repository::{Repository, RepositoryError},
};

pub mod book_created_producer;

#[derive(Clone)]
pub struct Service {
    repo: Repository,
    book_created_producer: Arc<BookCreatedProducer>,
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Repository error")]
    RepositoryError(#[from] RepositoryError),

    #[error("BookBuilderError error")]
    BookBuilderError(#[from] BookBuilderError),

    #[error("BookCreatedProducerError error")]
    BookCreatedProducerError(#[from] BookCreatedProducerError),
}

impl Service {
    pub fn new(repo: Repository, book_created_producer: BookCreatedProducer) -> Self {
        Self {
            repo,
            book_created_producer: Arc::new(book_created_producer),
        }
    }

    pub async fn create_book(&self, title: String, isbn: String) -> Result<Book, ServiceError> {
        let span = info_span!("create and publish book");

        let created_book_model = async move {
            let m = self
                .repo
                .create_book(title, isbn)
                .await
                .map_err(|e| ServiceError::RepositoryError(e));
            m
        }
        .instrument(span)
        .await?;

        let producer = self.book_created_producer.clone();
        let created_book_id = created_book_model.clone().id;
        let created_book_title = created_book_model.clone().title;
        let created_book_isbn = created_book_model.clone().isbn;
        let _ = tokio::task::spawn(async move {
            let _ = producer.publish_created_book(
                created_book_id,
                created_book_title,
                created_book_isbn,
            );
        })
        .instrument(info_span!("publish_created_book"));

        let book = BookBuilder::default()
            .id(created_book_model.id)
            .title(created_book_model.title)
            .isbn(created_book_model.isbn)
            .build()
            .map_err(|e| ServiceError::BookBuilderError(e))?;
        Ok(book)
    }
}
