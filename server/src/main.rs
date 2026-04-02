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
        let stream = stream?;  // Term if OS error, strips result
        let addr = stream.peer_addr()?;  // Term if OS drops conn before we can read peer
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
                        let msg = String::from_utf8_lossy(&payload);
                        println!("[{}] Says: {}", addr, msg);

                        let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");

                        clients_lock.retain_mut(|client| {
                            let target_addr = client.peer_addr().ok();

                            // Don't broadcast to sender
                            if let Some(ta) = target_addr {
                                if ta == addr { return true; }
                            }

                            match send_packet(client, PacketType::Message, &payload) {
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
                        println!("[{}] Disconnected", addr);
                        break;
                    }
                }
            }
        });
    }
    Ok(())
}
