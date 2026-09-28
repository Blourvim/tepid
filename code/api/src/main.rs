pub mod bloom;

use std::{
    io::{self, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    thread,
};

fn main() -> io::Result<()> {
    let backend_listener = TcpListener::bind("0.0.0.0:3002")?;

    thread::spawn(move || {
        for stream in backend_listener.incoming().flatten() {
            thread::spawn(move || {
                if let Err(e) = backend_handle(stream) {
                    eprintln!("backend error: {e}");
                }
            });
        }
    });

    let listener = TcpListener::bind("0.0.0.0:3001")?;
    for client in listener.incoming().flatten() {
        thread::spawn(move || {
            if let Err(e) = handle(client) {
                eprintln!("proxy error: {e}");
            }
        });
    }

    Ok(())
}

fn handle(client: TcpStream) -> io::Result<()> {
    let mut backend = TcpStream::connect("127.0.0.1:3002")?;

    let mut req_reader = client.try_clone()?;   
    let mut req_writer = backend.try_clone()?; 

    thread::spawn(move || {
        let _ = io::copy(&mut req_reader, &mut req_writer);
        let _ = req_writer.shutdown(Shutdown::Write); 
    });

    let mut resp_reader = backend;
    let mut resp_writer = client;
    let _ = io::copy(&mut resp_reader, &mut resp_writer);

    let _ = resp_writer.shutdown(Shutdown::Write);

    Ok(())
}

fn backend_handle(mut stream: TcpStream) -> io::Result<()> {
    let response = "hello";
    stream.write_all(response.as_bytes())?;

    stream.shutdown(Shutdown::Write)?; 
    let mut buf = [0u8; 1024];
    while stream.read(&mut buf)? > 0 {}

    Ok(())
}
