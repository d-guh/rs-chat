use std::io::{self, Write};
use std::net::TcpStream;
use std::thread;
use common::{receive_packet, send_packet, PacketType};

fn main() -> std::io::Result<()> {
    const ADDR: &str = "127.0.0.1";
    const PORT: u16 = 6767;

    let mut stream = TcpStream::connect((ADDR, PORT))?;
    println!("Connected to server at {}:{}", ADDR, PORT);
    println!("Type your message and press Enter. (Ctrl+C to quit)\n");

    let mut reader_stream = stream.try_clone()?;  // Term if fd limit

    // TODO: Add TUI for avoiding clearing user input when messages come in.
    // Debated adding Arc+Mutex for input, but mostly useless unless you enable crossterm raw mode anyways

    // Listener thread
    thread::spawn(move || {
        loop {
            match receive_packet(&mut reader_stream) {
                Ok((_header, payload)) => {
                    let msg = String::from_utf8_lossy(&payload);
                    print!("\r\x1b[2K[Incoming]: {}\n> ", msg);  // Clear prompt to print msg (ANSI jank)
                    io::stdout().flush().unwrap();
                }
                Err(_) => {
                    println!("\nConnection lost or server shut down.");
                    std::process::exit(0);
                }
            }
        }
    });

    // Sender loop (main thread)
    let mut input = String::new();
    loop {
        print!("> ");
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;

        let msg = input.trim();
        if !msg.is_empty() {
            if let Err(e) = send_packet(&mut stream, PacketType::Message, msg.as_bytes()) {
                eprintln!("Failed to send message: {}", e);
                break;
            }
        }
    }

    Ok(())
}
