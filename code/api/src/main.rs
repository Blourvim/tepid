pub mod bloom;
use std::{env, error::Error, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, copy},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend};
use tokio::net::{TcpListener, TcpStream};

use crate::bloom::Bloom;

pub async fn hydrate(db: DatabaseConnection) -> Result<Bloom, Box<dyn Error>> {
    let mut bloom_filter = Bloom::new(2_000_000);

    //todo: partial hydration here to make startup faster and easier for development,
    //should be replaced with a enviroment flag
    let rows = db
        .query_all(sea_orm::Statement::from_string(
            DbBackend::Postgres,
            "SELECT key_hash FROM revoked_keys LIMIT 100",
        ))
        .await?;

    for (i, row) in rows.iter().enumerate() {
        let token: String = row
            .try_get("", "key_hash")
            .map_err(|e| format!("failed to read 'token' column: {e}"))?;
        if i == 0 {
            println!("{:?}", &token)
        }
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

async fn handle(client: TcpStream, bloom_filter: &Bloom) -> std::io::Result<()> {
    let (client_read, mut client_write) = client.into_split();
    let mut reader = BufReader::new(client_read);

    let header = read_header(&mut reader).await?;

    let banned = match extract_authorization_hash(&header) {
        Some(value) => {
            let key = value
                .strip_prefix("Bearer ")
                .map(str::to_owned)
                .unwrap_or(value);
            println!("auth: {key:?}");

            // todo: should call the db here to check,
            bloom_filter.find(&key)
        }
        None => true,
    };

    if banned {
        refuse(reader, client_write).await;
        return Ok(());
    }

    let backend = TcpStream::connect("127.0.0.1:3002").await?;
    let (mut backend_read, mut backend_write) = backend.into_split();

    backend_write.write_all(&header).await?;

    let c2b = tokio::spawn(async move {
        let _ = copy(&mut reader, &mut backend_write).await;
        let _ = backend_write.shutdown().await;
    });

    let _ = copy(&mut backend_read, &mut client_write).await;
    let _ = client_write.shutdown().await;

    let _ = c2b.await;

    Ok(())
}

async fn refuse(mut reader: BufReader<OwnedReadHalf>, mut writer: OwnedWriteHalf) {
    const RESPONSE: &[u8] =
        b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    let _ = writer.write_all(RESPONSE).await;
    let _ = writer.shutdown().await;

    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        let mut buf = [0u8; 1024];
        while reader.read(&mut buf).await.unwrap_or(0) > 0 {}
    })
    .await;
}
async fn read_header<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
) -> std::io::Result<Vec<u8>> {
    let mut head = Vec::with_capacity(1024);
    let mut line = Vec::with_capacity(256);

    loop {
        line.clear();
        let n = reader.read_until(b'\n', &mut line).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "client closed before finishing headers",
            ));
        }
        head.extend_from_slice(&line);

        if line == [b'\r', b'\n'] || line == [b'\n'] {
            return Ok(head);
        }
    }
}

fn extract_authorization_hash(head: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(head).ok()?;
    for line in text.split('\n').skip(1) {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some((name, value)) = line.split_once(':') {
            if name.trim().eq_ignore_ascii_case("authorization") {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

async fn backend_handle(mut stream: TcpStream) -> std::io::Result<()> {
    let response = "hello";
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    let mut buf = [0u8; 1024];
    while stream.read(&mut buf).await? > 0 {}

    Ok(())
}
