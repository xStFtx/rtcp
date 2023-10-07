use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::Mutex;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = ServerConfig::new("127.0.0.1", 8080, 4);

    let listener = TcpListener::bind(format!("{}:{}", config.address, config.port))
        .await?;

    println!("Server listening on {}:{}", config.address, config.port);

    // Create a shared counter for active connections
    let active_connections = Arc::new(Mutex::new(0));

    // Create a signal handler for graceful shutdown
    let sigint = signal::ctrl_c();

    // Start the signal handler in a separate task
    tokio::spawn(async move {
        sigint.await.expect("Failed to install signal handler");
        println!("Received Ctrl+C. Shutting down gracefully.");
        std::process::exit(0);
    });

    // Create a thread pool for handling connections
    let mut handles = vec![];

    let listener = Arc::new(listener);

    for _ in 0..config.max_threads {
        let listener = Arc::clone(&listener);
        let active_connections = Arc::clone(&active_connections);

        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let active_connections = Arc::clone(&active_connections);

                // Check the number of active connections and limit them
                if *active_connections.lock().await < config.max_threads as i32 {
                    // Spawn a new task to handle the connection
                    tokio::spawn(handle_client(stream, active_connections));
                } else {
                    // Reject the connection if the limit is reached
                    println!("Connection limit reached, rejecting a connection.");
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all worker threads to finish
    for handle in handles {
        handle.await?;
    }

    Ok(())
}

// Configuration struct for the server
struct ServerConfig {
    address: String,
    port: u16,
    max_threads: usize,
}

impl ServerConfig {
    fn new(address: &str, port: u16, max_threads: usize) -> Self {
        ServerConfig {
            address: address.to_string(),
            port,
            max_threads,
        }
    }
}

async fn handle_client(mut stream: tokio::net::TcpStream, active_connections: Arc<Mutex<i32>>) {
    let mut buffer = vec![0; 1024]; // Buffer to read incoming data into

    let peer_addr = stream.peer_addr().unwrap();
    println!("Accepted connection from: {}", peer_addr);

    {
        let mut active_connections = active_connections.lock().await;
        *active_connections += 1;
    }

    // Clone the stream for reading and writing
    let (mut reader, mut writer) = tokio::io::split(stream);

    loop {
        match reader.read(&mut buffer).await {
            Ok(0) => {
                // Connection closed
                break;
            }
            Ok(n) => {
                // Handle the data here
                let message = String::from_utf8_lossy(&buffer[..n]);
                println!("Received from {}: {}", peer_addr, message);
                // Echo the data back to the client
                if let Err(err) = writer.write_all(&buffer[..n]).await {
                    eprintln!("Error writing to client: {}", err);
                    break;
                }
                if let Err(err) = writer.flush().await {
                    eprintln!("Error flushing writer: {}", err);
                    break;
                }
            }
            Err(err) => {
                eprintln!("Error reading from client: {}", err);
                break;
            }
        }
    }

    {
        let mut active_connections = active_connections.lock().await;
        *active_connections -= 1;
    }

    println!("Connection closed from: {}", peer_addr);
}
