use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

type Callback = Box<dyn Fn(String) + Send + Sync>;

#[derive(Eq, PartialEq, Hash)]
pub enum CallbackType {
    CommandMessage = 0x01,
    ServerMessage = 0x02,
    ServerErrorMessage = 0xFF,
}

#[derive(Clone)]
pub struct CallbackManager {
    callbacks: Arc<Mutex<HashMap<CallbackType, Callback>>>,
}

impl CallbackManager {
    // Create a new CallbackManager
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Add a callback to be invoked later
    pub async fn register_callback(&self, event: CallbackType, callback: Callback) {
        let mut callbacks = self.callbacks.lock().await;
        callbacks.insert(event, callback);
    }

    // Invoke the callback for the event
    pub async fn invoke(&self, event: CallbackType, con: String) {
        if let Some(callback) = self.callbacks.lock().await.get(&event) {
            callback(con);
        }
    }

    pub(crate) async fn handle_command_server_message(
        &self,
        buf: &[u8; 51200],
        amt: usize,
    ) {
        let message = String::from_utf8_lossy(&buf[9..amt]).to_string();
        self.invoke(CallbackType::CommandMessage, message).await;
    }

    pub(crate) async fn handle_server_message(
        &self,
        socket: Arc<Mutex<UdpSocket>>,
        buf: &[u8],
        amt: usize,
        server_addr: &SocketAddr,
        sequence: u8,
    ) {
        let message = String::from_utf8_lossy(&buf[9..amt]).to_string();
        self.invoke(CallbackType::ServerMessage, message).await;
        if let Err(e) = crate::connection_manager::send_ack(socket, server_addr, sequence).await {
            self.invoke(CallbackType::ServerErrorMessage, e).await;
        }
    }
}