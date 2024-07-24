use crate::entity::book::{ActiveModel as BookActiveModel, Model as BookModel};
use migration::sea_orm::{ConnectOptions, Database};
use migration::{Migrator, MigratorTrait};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr};
use std::sync::Arc;
use thiserror::Error;

#[derive(Clone)]
pub struct Repository {
    database_connection: Arc<DatabaseConnection>,
}

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Database error")]
    DatabaseError(#[from] DbErr),
}

impl Repository {
    pub async fn new(database_connection: DatabaseConnection) -> Result<Self, RepositoryError> {
        // Migrator::up(&database_connection, None)
        //     .await
        //     .map_err(|e| RepositoryError::DatabaseError(e))?;
        Ok(Self {
            database_connection: Arc::new(database_connection),
        })
    }

    pub async fn create_book(
        &self,
        title: String,
        isbn: String,
    ) -> Result<BookModel, RepositoryError> {
        let created_book = BookActiveModel {
            title: Set(title),
            isbn: Set(isbn),
            ..Default::default()
        };
        created_book
            .insert(self.database_connection.as_ref())
            .await
            .map_err(|e| RepositoryError::DatabaseError(e))
    }
}

// Tests will fail migration needs to be run do later
#[cfg(test)]
mod test {
    use database::get_connection;
    use testcontainers::{clients, images};

    use crate::repository::Repository;

    #[tokio::test]
    async fn test_create_book() {
        let docker = clients::Cli::default();
        let db = images::postgres::Postgres::default();
        let node = docker.run(db);
        dbg!(&format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
        ));
        let conn_string = &format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
        );
        let db_conn = get_connection(conn_string).await.unwrap();
        let repo = Repository::new(db_conn.clone()).await.unwrap();
        let title = "TITILE".to_string();
        let isbn = "ISDB".to_string();
        let created_book = repo.create_book(title.clone(), isbn.clone()).await.unwrap();
        assert_eq!(created_book.title, title);
        assert_eq!(created_book.isbn, isbn);
    }

    #[tokio::test]
    async fn test_create_book_uniqe() {
        let docker = clients::Cli::default();
        let db = images::postgres::Postgres::default();
        let node = docker.run(db);
        dbg!(&format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
        ));
        let conn_string = &format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            node.get_host_port_ipv4(5432)
        );
        let db_conn = get_connection(conn_string).await.unwrap();
        let repo = Repository::new(db_conn.clone()).await.unwrap();
        let title = "TITILE".to_string();
        let isbn = "ISDB".to_string();
        let created_book = repo.create_book(title.clone(), isbn.clone()).await.unwrap();
        assert_eq!(created_book.title, title);
        assert_eq!(created_book.isbn, isbn);

        let created_book2_result = repo.create_book(title.clone(), isbn.clone()).await;
        assert!(created_book2_result.is_err());
    }
}
