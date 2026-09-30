pub mod bloom;

use std::{env, error::Error, time::Duration};

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, copy},
    net::{TcpListener, TcpStream},
};

use crate::bloom::Bloom;

pub async fn hydrate(db: DatabaseConnection) -> Result<Bloom, Box<dyn Error>> {
    let mut bloom_filter = Bloom::new(2_000_000);
    let rows = db
        .query_all(sea_orm::Statement::from_string(
            DbBackend::Postgres,
            "SELECT key_hash FROM revoked_keys",
        ))
        .await?;

    for row in rows {
        let token: String = row
            .try_get("", "key_hash")
            .map_err(|e| format!("failed to read 'token' column: {e}"))?;
        bloom_filter.push(&token);
    }

    println!("Hydration complete");
    Ok(bloom_filter)
}
async fn start_db(connection_string: &str) -> Result<DatabaseConnection, sea_orm::DbErr> {

    let mut opt = sea_orm::ConnectOptions::new(connection_string);
    opt.max_connections(100)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8))
        .sqlx_logging(false);

    let db = sea_orm::Database::connect(opt).await?;
    Ok(db)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db_connection = start_db(&database_url)
        .await
        .expect("Database connection failed");

    let bloom_filter = hydrate(db_connection).await?;

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
                tokio::spawn(async move {
                    if let Err(e) = handle(client).await {
                        eprintln!("proxy error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("proxy accept error: {e}"),
        }
    }
}

async fn handle(client: TcpStream) -> std::io::Result<()> {
    let backend = TcpStream::connect("127.0.0.1:3002").await?;

    // tokio::net::TcpStream has no try_clone(); use into_split for owned halves.
    let (mut client_read, mut client_write) = client.into_split();
    let (mut backend_read, mut backend_write) = backend.into_split();

    // client -> backend
    let c2b = tokio::spawn(async move {
        let _ = copy(&mut client_read, &mut backend_write).await;
        let _ = backend_write.shutdown().await;
    });

    // backend -> client
    let _ = copy(&mut backend_read, &mut client_write).await;
    let _ = client_write.shutdown().await;

    let _ = c2b.await;

    Ok(())
}

async fn backend_handle(mut stream: TcpStream) -> std::io::Result<()> {
    let response = "hello";
    stream.write_all(response.as_bytes()).await?;

    stream.shutdown().await?;
    let mut buf = [0u8; 1024];
    while stream.read(&mut buf).await? > 0 {}

    Ok(())
}

