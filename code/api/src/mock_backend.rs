use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;

pub async fn backend_listen() -> std::io::Result<()> {
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

    Ok(())

}

pub async fn backend_handle(mut stream: TcpStream) -> std::io::Result<()> {
    let response = "hello";
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    let mut buf = [0u8; 1024];
    while stream.read(&mut buf).await? > 0 {}

    Ok(())
}
