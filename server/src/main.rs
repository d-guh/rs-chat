use std::net::{TcpListener, TcpStream, SocketAddr};
use std::sync::{Arc, Mutex};
use std::thread;
use common::{receive_packet, send_packet, PacketType, Client, MAX_USERNAME_LEN};

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
            let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");
            let new_client = Client::new(stream.try_clone()?, addr);  // Term if fd/socket limit
            clients_lock.push(new_client);
        }

        // Thread for each client
        thread::spawn(move || { handle_client(stream, addr, clients_clone); });  // Closure here calls function rather than function returning closure for simplicity
    }
    Ok(())
}

fn handle_client(mut stream: TcpStream, addr: SocketAddr, clients_clone: Arc<Mutex<Vec<Client>>>) {
    loop {
        match receive_packet(&mut stream) {
            Ok((header, payload)) => {
                if header.packet_type == PacketType::System {
                    eprintln!("WARNING: Client [{}] attempted to spoof System packet. Dropping.", addr);
                    continue;
                }

                let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");
                let username = get_username(&clients_lock, addr);

                match header.packet_type {
                    PacketType::Message => {
                        let msg_content = String::from_utf8_lossy(&payload);
                        log_info(addr, &username, &format!("Says: {}", msg_content));

                        let broadcast_msg = format!("[{}]: {}", username, msg_content);
                        broadcast_message(&mut clients_lock, addr, &broadcast_msg);
                    }
                    PacketType::Login => {
                        let mut new_name = String::from_utf8_lossy(&payload).trim().to_string();

                        if new_name.len() > MAX_USERNAME_LEN {
                            new_name.truncate(MAX_USERNAME_LEN);
                        }

                        if let Some(c) = clients_lock.iter_mut().find(|c| c.addr == addr) {
                            c.username = new_name.clone();
                            log_info(addr, &new_name, "Identified.");
                            
                            let welcome_msg = format!("{} has joined the chat.", new_name);
                            broadcast_system_message(&mut clients_lock, &welcome_msg);
                        }
                    }
                    PacketType::Quit => break,
                    PacketType::Heartbeat => {
                        // Server responds to clients checking in
                        // Could be improved
                        let _ = send_packet(&mut stream, PacketType::Heartbeat, &[]);
                    }
                    PacketType::Command => {
                        log_info(addr, &username, "Command received.");
                    }
                    _ => {}  // Do nothing for other PacketType
                }
            }
            Err(_) => break,
        }
    }

    // Cleanup (when loop breaks this executes)
    remove_client(addr, clients_clone);
}


fn remove_client(addr: SocketAddr, clients_clone: Arc<Mutex<Vec<Client>>>) {
    let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");
    
    let username = get_username(&clients_lock, addr);
    clients_lock.retain(|c| c.addr != addr);

    log_info(addr, &username, "Disconnected.");
    
    let leave_msg = format!("{} has left the chat.", username);
    broadcast_system_message(&mut clients_lock, &leave_msg);
}

fn log_info(addr: SocketAddr, username: &str, message: &str) {
    println!("[{} ({})] {}", addr, username, message);
}

fn get_username(clients: &[Client], addr: SocketAddr) -> String {
    clients.iter()
        .find(|c| c.addr == addr)
        .map(|c| c.username.clone())
        .unwrap_or_else(|| addr.to_string())  // If username fails, fallback to IP
}

fn broadcast_message(clients: &mut Vec<Client>, sender_addr: SocketAddr, message: &str) {
    let payload = message.as_bytes();
    clients.retain_mut(|client| {
        if client.addr == sender_addr { return true; }

        match send_packet(&mut client.stream, PacketType::Message, payload) {
            Ok(_) => true,
            Err(_) => {
                println!("!> Failed to broadcast to {}. Removing.", client.addr);
                false
            }
        }
    });
}

fn broadcast_system_message(clients: &mut Vec<Client>, message: &str) {
    let payload = message.as_bytes();
    clients.retain_mut(|client| {
        match send_packet(&mut client.stream, PacketType::System, payload) {
            Ok(_) => true,
            Err(_) => false,
        }
    });
}

// Forcefully kill client connection, not with Quit packet, but at TCP level
fn _kill_client(addr: SocketAddr, clients_clone: Arc<Mutex<Vec<Client>>>) {
    let mut clients_lock = clients_clone.lock().expect("Failed to lock mutex");
    if let Some(pos) = clients_lock.iter().position(|c| c.addr == addr) {
        let client = clients_lock.remove(pos);

        let _ = client.stream.shutdown(std::net::Shutdown::Both);
        println!("[ADMIN] Forcefully killed connection: {}", addr);
    }
}
