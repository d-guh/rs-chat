use std::net::TcpStream;
use common::{send_packet, PacketType};

fn main() -> std::io::Result<()> {
    const ADDR: &str = "127.0.0.1";
    const PORT: u16 = 6767;

    let mut stream = TcpStream::connect((ADDR, PORT))?;
    println!("Connected to server at {}:{}", ADDR, PORT);

    let message = "Hello from client!";
    send_packet(&mut stream, PacketType::Message, message.as_bytes())?;

    println!("Message sent successfully.");
    Ok(())
}
