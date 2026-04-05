use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use common::{receive_packet, send_packet, PacketType, Client};

fn main() -> std::io::Result<()> {
    const ADDR: &str = "127.0.0.1";
    const PORT: u16 = 6767;

    let listener = TcpListener::bind((ADDR, PORT))?;  // Term if can't open port
    let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));

    println!("Server listening on {}:{}", ADDR, PORT);

    // Incoming connections loop
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;  // Skip if OS-level conn error
            }
        };
        let addr = match stream.peer_addr() {
            Ok(a) => a,
            Err(e) => {
                eprintln!("Client disconnected before handshake: {}", e);
                continue;  // Skip if immediate RST or error
            }
        };

        println!("[{}] Incoming Connection", addr);

        let clients_clone = Arc::clone(&clients);

        {
            let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");  // Term if fail to lock
            let new_client = Client::new(stream.try_clone()?, addr);  // Term if fd/socket limit
            clients_lock.push(new_client);
        }

        // Thread for each client
        thread::spawn(move || { handle_client(stream, addr, clients_clone); });  // Closure here calls function rather than function returning closure for simplicity
    }
    Ok(())
}

fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    clients_clone: Arc<Mutex<Vec<Client>>>
) {
    loop {
        match receive_packet(&mut stream) {
            Ok((header, payload)) => {
                let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");  // Term if fail to lock

                match header.packet_type {
                    PacketType::Message => {
                        let msg_content = String::from_utf8_lossy(&payload);

                        let sender_name = clients_lock.iter()
                            .find(|c| c.addr == addr)
                            .map(|c| c.username.clone())
                            .unwrap_or_else(|| addr.to_string());  // If username fails, fallback to IP

                        println!("[{}] Says: {}", sender_name, msg_content);

                        let broadcast_msg = format!("[{}]: {}", sender_name, msg_content);
                        let broadcast_payload = broadcast_msg.as_bytes();

                        clients_lock.retain_mut(|client| {
                            if client.addr == addr { return true; }

                            match send_packet(&mut client.stream, PacketType::Message, broadcast_payload) {
                                Ok(_) => {
                                    println!("  => Broadcast to {}", client.addr);
                                    true
                                }
                                Err(_) => {
                                    println!("  !> Connection lost with {}. Removing.", client.addr);
                                    false
                                }
                            }
                        });
                    }
                    PacketType::Login => {
                        let username = String::from_utf8_lossy(&payload).trim().to_string();
                        if let Some(c) = clients_lock.iter_mut().find(|c| c.addr == addr) {
                            c.username = username.clone();
                            println!("[{}] Identified as: {}", addr, username);
                        }
                    }
                    _ => {}  // TODO: Heartbeat, command, etc.
                }
            }
            Err(_) => {
                println!("[{}] Disconnected.", addr);
                if let Ok(mut clients_lock) = clients_clone.lock() {
                    clients_lock.retain(|c| c.addr != addr);
                }
                break;
            }
        }
    }
}
