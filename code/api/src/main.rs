pub mod bloom;
pub mod db;
pub mod mock_backend;
pub mod handler;
pub mod http;
use crate::{handler::handle, mock_backend::backend_handle};
use std::{env, error::Error, sync::Arc};

use tokio::net::TcpListener;

use crate::db::{hydrate, start_db};


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_connection = start_db(&database_url)
        .await
        .expect("Database connection failed");

    let bloom_filter = Arc::new(hydrate(db_connection).await?);

    let backend_listener = TcpListener::bind("0.0.0.0:3002").await?;

    tokio::spawn(async move {
        loop {
            match backend_listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(async move {
                        if let Err(e) = backend_handle(stream).await {
                            eprintln!("backend error: {e}");
                        }
                    });
                }
                Err(e) => eprintln!("backend accept error: {e}"),
            }
        }
    });

    let listener = TcpListener::bind("0.0.0.0:3001").await?;
    loop {
        match listener.accept().await {
            Ok((client, _)) => {
                let bloom = Arc::clone(&bloom_filter);
                tokio::spawn(async move {
                    if let Err(e) = handle(client, &bloom).await {
                        eprintln!("proxy error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("proxy accept error: {e}"),
        }
    }
}
