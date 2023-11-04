use crate::callback::CallbackManager;
use crate::packet::{create_command_packet, create_login_packet, create_server_message_ack_packet};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time;

#[derive(Clone)]
pub struct RConClient {
    socket: Arc<Mutex<UdpSocket>>,
    server_addr: SocketAddr,
    send_pending: Arc<AtomicBool>,
    pub callback_manager: CallbackManager,
}

impl RConClient {
    pub async fn new(server_addr: &str, bind_addr: &str) -> Result<Self, Box<dyn Error>> {
        let socket = UdpSocket::bind(bind_addr).await?;
        let server_addr: SocketAddr = server_addr.parse()?;

        Ok(Self {
            socket: Arc::new(Mutex::new(socket)),
            server_addr,
            send_pending: Arc::new(AtomicBool::new(false)),
            callback_manager: CallbackManager::new(),
        })
    }

    pub async fn login(&self, password: &str) -> Result<bool, Box<dyn Error>> {
        let login_packet = create_login_packet(password);
        let socket = self.socket.lock().await;
        socket.send_to(&login_packet, &self.server_addr).await?;

        let mut buf = [0u8; 1024];
        let (amt, _) = socket.recv_from(&mut buf).await?;
        if amt > 0 && buf[7] == 0x00 {
            return Ok(buf[8] == 0x01);
        }
        self.send_pending.store(false, Ordering::Release);
        Ok(false)
    }

    pub async fn send_keep_alive_packet(&self) -> Result<(), String> {
        let packet = create_command_packet(0, "");
        let socket = self.socket.lock().await;
        if let Err(err) = socket.send_to(&packet, &self.server_addr).await {
            return Err(err.to_string())
        };
        Ok(())
    }

    pub async fn start_keep_alive_task(&self) {
        let client_clone = self.clone();
        let callback_manager_clone = self.callback_manager.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(15));
            loop {
                interval.tick().await;
                if let Err(e) = client_clone.send_keep_alive_packet().await {
                    callback_manager_clone.handle_error(e.to_string()).await;
                }
            }
        });
    }

    pub async fn send_command(&self, command: &str) -> Result<(), Box<dyn Error>> {
        self.send_pending.store(true, Ordering::Release);
        let sequence = 0;

        let command_packet = create_command_packet(sequence, command);

        let socket = {
            let sock_lock = self.socket.lock().await;
            sock_lock
        };
        socket.send_to(&command_packet, &self.server_addr).await?;
        self.send_pending.store(false, Ordering::Release);
        Ok(())
    }

    pub async fn start_listening(&mut self) -> Result<(), Box<dyn Error>> {
        let socket_clone = self.socket.clone();
        let server_addr_clone = self.server_addr;
        let send_pending_clone = self.send_pending.clone();
        let callback_manager_clone = self.callback_manager.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                if send_pending_clone.load(Ordering::Acquire) {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    continue;
                }

                let socket_guard = socket_clone.lock().await;
                let mut buf = [0u8; 1024 * 50]; // 50 kb

                let time_out_duration;
                if send_pending_clone.load(Ordering::Acquire) {
                    time_out_duration = Duration::from_millis(500);
                } else {
                    time_out_duration = Duration::from_millis(100);
                }
                let result =
                    time::timeout(time_out_duration, socket_guard.recv_from(&mut buf)).await;

                drop(socket_guard);
                match result {
                    Ok(Ok((amt, _))) => {
                        if amt > 9 {
                            let response_type = buf[7];
                            match response_type {
                                0x02 => {
                                    callback_manager_clone
                                        .handle_server_message(
                                            socket_clone.clone(),
                                            &buf,
                                            amt,
                                            &server_addr_clone,
                                            0,
                                        )
                                        .await;
                                }
                                0x01 | 0x00 => {
                                    callback_manager_clone
                                        .handle_command_server_message(&buf, amt)
                                        .await;
                                }
                                _ => {
                                    todo!()
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        eprintln!("Failed to receive data: {}", e);
                        continue;
                    }
                    Err(_) => {
                        continue;
                    }
                }
            }
        });
        Ok(())
    }
}

pub(crate) async fn send_ack(
    socket: Arc<Mutex<UdpSocket>>,
    server_addr: &SocketAddr,
    sequence: u8,
) -> Result<(), String> {
    let ack_packet = create_server_message_ack_packet(sequence);

    let socket_guard = socket.lock().await;
    if let Err(err) = socket_guard.send_to(&ack_packet, server_addr).await {
        return Err(err.to_string());
    };

    Ok(())
}
