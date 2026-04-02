use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use common::{receive_packet, send_packet, PacketType};

fn main() -> std::io::Result<()> {
    const ADDR: &str = "127.0.0.1";
    const PORT: u16 = 6767;

    let listener = TcpListener::bind((ADDR, PORT))?;  // Term if can't open port
    let clients: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));

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
            let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");
            clients_lock.push(stream.try_clone()?);  // Term if fd/socket limit
        }

        // Thread for each client
        thread::spawn(move || {
            let mut stream = stream;
            loop {
                match receive_packet(&mut stream) {
                    Ok((_header, payload)) => {
                        let msg_content = String::from_utf8_lossy(&payload);
                        println!("[{}] Says: {}", addr, msg_content);

                        let msg = format!("[{}]: {}", addr, msg_content);
                        let broadcast_payload = msg.as_bytes();

                        let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");

                        clients_lock.retain_mut(|client| {
                            let target_addr = client.peer_addr().ok();

                            // Don't broadcast to sender
                            if let Some(ta) = target_addr {
                                if ta == addr { return true; }
                            }

                            match send_packet(client, PacketType::Message, broadcast_payload) {
                                Ok(_) => {
                                    if let Some(ta) = target_addr { println!("  => Broadcast to {}", ta); }
                                    true
                                }
                                Err(_) => {
                                    if let Some(ta) = target_addr { println!("  !> Connection lost with {}. Removing.", ta); }
                                    false
                                }
                            }
                        });
                    }
                    Err(_) => {
                        println!("[{}] Disconnected. Removing.", addr);

                        // Immediately remove disconnected client
                        if let Ok(mut clients_lock) = clients_clone.lock() {
                            clients_lock.retain(|client| {
                                match client.peer_addr() {
                                    Ok(c_addr) => c_addr != addr,  // Remove disconnected client
                                    Err(_) => false,  // If unable to get addr, dead anyways
                                }
                            });
                        }
                        break;
                    }
                }
            }
        });
    }
    Ok(())
}
