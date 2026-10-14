pub mod bloom;
pub mod db;
pub mod mock_backend;
pub mod handler;
pub mod http;
pub mod ebpf;
use crate::{handler::handle, mock_backend::backend_listen};
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


    let _ =backend_listen().await;

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
