use tokio::io::AsyncBufReadExt;

pub async fn read_header<R: tokio::io::AsyncBufRead + Unpin>(
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

pub fn extract_authorization_hash(head: &[u8]) -> Option<String> {
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

