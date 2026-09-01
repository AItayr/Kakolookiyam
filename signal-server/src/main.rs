use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum Signal {
    Register { id: String },
    Unregister { id: String }, // Ordre de libération
    Offer { sdp: String, sender_id: String, target_id: String },
    ChatOffer { sdp: String, sender_id: String, target_id: String },
    Answer { sdp: String, sender_id: String, target_id: String },
    Ice { candidate: String, sender_id: String, target_id: String },
    Ping { msg: String }
}

type Clients = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

#[tokio::main]
async fn main() {
    // Écoute sur toutes les interfaces réseau du VPS Oracle
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await.expect("Impossible de lier le port");

    // Mode furtif strict : Aucun log console
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

    while let Ok((stream, _)) = listener.accept().await {
        let clients_clone = clients.clone();
        tokio::spawn(async move {
            if let Ok(ws_stream) = accept_async(stream).await {
                handle_connection(ws_stream, clients_clone).await;
            }
        });
    }
}

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    clients: Clients
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut client_id = String::new();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(tokio_tungstenite::tungstenite::protocol::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Ok(text) = msg.into_text() {
            if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                match signal {
                    Signal::Register { id } => {
                        let mut clients_guard = clients.lock().await;
                        if clients_guard.contains_key(&id) {
                            let _ = tx.send("ERROR:ALREADY_CONNECTED".to_string());
                        } else {
                            client_id = id.clone();
                            clients_guard.insert(id, tx.clone());
                            let _ = tx.send("SUCCESS:REGISTERED".to_string());
                        }
                    },
                    Signal::Unregister { id } => {
                        let mut clients_guard = clients.lock().await;
                        clients_guard.remove(&id);
                        if client_id == id {
                            client_id.clear();
                        }
                    },
                    Signal::Offer { target_id, .. } |
                    Signal::ChatOffer { target_id, .. } |
                    Signal::Answer { target_id, .. } |
                    Signal::Ice { target_id, .. } => {
                        let clients_guard = clients.lock().await;
                        if let Some(target_tx) = clients_guard.get(&target_id) {
                            let _ = target_tx.send(text.to_string());
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    // Nettoyage automatique en cas de déconnexion inattendue (Zéro-Trace)
    if !client_id.is_empty() {
        clients.lock().await.remove(&client_id);
    }

    send_task.abort();
}