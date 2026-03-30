use std::net::TcpListener;
use common::receive_packet;

fn main() -> std::io::Result<()> {
    const ADDR: &str = "127.0.0.1";
    const PORT: u16 = 6767;

    let listener = TcpListener::bind((ADDR, PORT))?;
    println!("Server listening on {}:{}", ADDR, PORT);

    for stream in listener.incoming() {
        let mut stream = stream?;
        println!("New client connected: {:?}", stream.peer_addr()?);

        match receive_packet(&mut stream) {
            Ok((header, payload)) => {
                let msg = String::from_utf8_lossy(&payload);

                println!("Received Header: {:?}", header);
                println!("Message: {}", msg);
            }
            Err(e) => eprintln!("Error reading packet: {}", e),
        }
    }
    Ok(())
}
