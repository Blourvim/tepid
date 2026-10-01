use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, copy},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};
use tokio::net::TcpStream;

use crate::{bloom::Bloom, http::{extract_authorization_hash, read_header}};

pub async fn handle(client: TcpStream, bloom_filter: &Bloom) -> std::io::Result<()> {
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

pub async fn refuse(mut reader: BufReader<OwnedReadHalf>, mut writer: OwnedWriteHalf) {
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
