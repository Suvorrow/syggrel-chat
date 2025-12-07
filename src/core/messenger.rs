use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio_socks::tcp::Socks5Stream;

pub struct YggdrasilMessenger {
    buffer: MessageBuffer,
    connection_handle: Option<JoinHandle<tokio::io::Result<()>>>,
    message_tx: Option<mpsc::UnboundedSender<String>>,
}

impl YggdrasilMessenger {
    pub fn new() -> Self {
        Self {
            buffer: MessageBuffer::new(),
            connection_handle: None,
            message_tx: None,
        }
    }

    pub async fn connect_via_socks5(
        &mut self,
        proxy_addr: &str,
        target_addr: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let proxy_addr = proxy_addr.parse()
            .map_err(|e| format!("Invalid proxy address '{}': {}", proxy_addr, e))?;
        let target_addr = target_addr.parse()
            .map_err(|e| format!("Invalid target address '{}': {}", target_addr, e))?;

        let stream = Socks5Stream::connect(proxy_addr, target_addr)
            .await
            .map_err(|e| format!("SOCKS5 connection failed: {}", e))?;

        // Create channel for receiving messages from the connection
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        self.message_tx = Some(tx);

        let buffer = self.buffer.buffer.clone();

        // Clone the buffer for the background task
        let handle = tokio::spawn(async move {
            // Split stream for concurrent read/write if needed
            let (mut reader, mut writer) = tokio::io::split(stream);

            // Task for receiving messages
            let recv_task = tokio::spawn(async move {
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();
                let buffer = buffer.clone();

                loop {
                    line.clear();
                    match buf_reader.read_line(&mut line).await {
                        Ok(0) => break, // EOF
                        Ok(_) => {
                            let msg = line.trim_end_matches(['\r', '\n']).to_string(); // Clean the message
                            if let Err(e) = {
                                let mut buffer_guard = buffer.lock().await;
                                buffer_guard.add_message(msg);
                                Ok(())
                            } {
                                eprintln!("Buffer error: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("Read error: {}", e);
                            break;
                        }
                    }
                }
            });
            // Task for sending messages via network
            let send_task = tokio::spawn(async move {
                while let Ok(msg) = rx.recv().await {
                    if let Err(e) = writer.write_all(msg.as_bytes()).await {
                        eprintln!("Write error: {}", e);
                        break;
                    }
                    // Add newline delimiter to separate messages
                    if let Err(e) = writer.write_all(b"\n").await {
                        eprintln!("Write error: {}", e);
                        break;
                    }
                    // Ensure data is sent immediately
                    if let Err(e) = writer.flush().await {
                        eprintln!("Flush error: {}", e);
                        break;
                    }
                }
            });

            // Wait for either task to complete
            tokio::select! {
                _ = recv_task => {},
                _ = send_task => {},
            }

            Ok(())
        });

        self.connection_handle = Some(handle);
        Ok(())
    }

    pub async fn send_message(&self, msg: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Send through the network connection
        if let Some(ref tx) = self.message_tx {
            tx.send(msg).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            Ok(())
        } else {
            Err("Not connected".into())
        }
    }

    pub async fn receive_messages(&self, count: usize) -> Vec<String> {
        self.buffer.take_messages(count).await 
    }

    pub fn queue_network_message(&self, msg: String) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref tx) = self.message_tx {
            tx.send(msg).map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        } else {
            Err("Not connected".into())
        }
    }

    // Disconnect and resource cleanup method
    pub async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(handle) = self.connection_handle.take() {
            handle.abort();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(1), handle).await;
        }

        self.message_tx = None;
        Ok(())
    }
}