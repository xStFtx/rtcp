use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::error::Error;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await?;
    let (tx, _rx) = broadcast::channel::<String>(10);
    let shared_tx = Arc::new(tx);

    println!("Server listening on {}", addr);

    while let Ok((socket, _addr)) = listener.accept().await {
        let shared_tx = Arc::clone(&shared_tx);

        tokio::spawn(async move {
            let result = handle_client(socket, shared_tx).await;
            if let Err(err) = result {
                eprintln!("Error handling client: {}", err);
            }
        });
    }

    Ok(())
}

async fn handle_client(mut socket: TcpStream, tx: Arc<broadcast::Sender<String>>) -> Result<(), Box<dyn Error>> {
    let mut buf = [0; 1024];
    let (mut reader, mut writer) = socket.split();

    while let Ok(n) = reader.read(&mut buf).await {
        if n == 0 {
            break; // Connection closed
        }

        let received_data = String::from_utf8_lossy(&buf[..n]);
        println!("Received: {}", received_data);

        // Broadcast the received data to all connected clients
        tx.send(received_data.to_string())?;

        // Echo the data back to the client
        writer.write_all(received_data.as_bytes()).await?;
        writer.flush().await?;
    }

    Ok(())
}
