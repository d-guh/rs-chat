use std::io::{self, Write};
use std::net::TcpStream;
use std::thread;
use common::{receive_packet, send_packet, PacketType};

fn main() -> io::Result<()> {
    const ADDR: &str = "127.0.0.1";
    const PORT: u16 = 6767;
    
    let mut stream = TcpStream::connect((ADDR, PORT))?;
    println!("Connected to server at {}:{}", ADDR, PORT);

    perform_login(&mut stream)?;

    let rx_stream = stream.try_clone()?;
    thread::spawn(move || { handle_receiver(rx_stream); });

    handle_sender(stream)?;

    Ok(())
}

fn perform_login(stream: &mut TcpStream) -> io::Result<()> {
    print!("Enter username: ");
    io::stdout().flush()?;

    let mut username = String::new();
    io::stdin().read_line(&mut username)?;
    let username = username.trim();

    if username.is_empty() {
        send_packet(stream, PacketType::Login, b"Anonymous")
    } else {
        send_packet(stream, PacketType::Login, username.as_bytes())
    }
}

fn handle_receiver(mut stream: TcpStream) {
    loop {
        match receive_packet(&mut stream) {
            Ok((header, payload)) => {
                let msg = String::from_utf8_lossy(&payload);
                
                match header.packet_type {
                    PacketType::Message => {
                        println!("{}", msg);
                    }
                    PacketType::System => {
                        println!("[SYSTEM] {}", msg);
                    }
                    PacketType::Quit => {
                        println!("Server requested disconnect.");
                        break;
                    }
                    PacketType::Heartbeat => {
                        // TODO: Implement Heartbeat
                    }
                    _ => {} // Ignore other types
                }
            }
            Err(_) => {
                println!("\nLost connection to server.");
                break;
            }
        }
    }
    std::process::exit(0);
}

fn handle_sender(mut stream: TcpStream) -> io::Result<()> {
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed == "/quit" {
            send_packet(&mut stream, PacketType::Quit, b"")?;
            println!("Goodbye!");
            break;
        }

        if let Err(e) = send_packet(&mut stream, PacketType::Message, trimmed.as_bytes()) {
            eprintln!("Failed to send message: {}", e);
            break;
        }
    }
    Ok(())
}
