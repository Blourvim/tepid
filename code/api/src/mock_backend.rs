use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub async fn backend_handle(mut stream: TcpStream) -> std::io::Result<()> {
    let response = "hello";
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    let mut buf = [0u8; 1024];
    while stream.read(&mut buf).await? > 0 {}

    Ok(())
}

