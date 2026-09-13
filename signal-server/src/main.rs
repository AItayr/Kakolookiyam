use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};

use hmac::{Hmac, Mac};
use sha1::Sha1;
use base64::Engine;

type HmacSha1 = Hmac<Sha1>;

fn generate_turn_credentials(user_id: &str) -> (String, String) {
    // Le secret est maintenant lu depuis une variable d'environnement pour l'Open Source !
    // Si la variable TURN_SECRET n'est pas définie sur le serveur, on utilise un mot de passe public par défaut.
    let secret = std::env::var("TURN_SECRET").unwrap_or_else(|_| "DEFAULT_KAKO_SECRET_OPENSOURCE".to_string());
    let unix_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let expiration = unix_time + 7200;
    let username = format!("{}:{}", expiration, user_id);
    let mut mac = HmacSha1::new_from_slice(secret.as_bytes()).expect("HMAC err");
    mac.update(username.as_bytes());
    let result = mac.finalize().into_bytes();
    let password = base64::engine::general_purpose::STANDARD.encode(result);
    (username, password)
}

fn verify_signature(pub_id_hex: &str, pseudo: &str, timestamp: u64, signature_hex: &str) -> bool {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    if timestamp < now - 60 || timestamp > now + 60 { return false; }
    
    let mut pub_key_bytes = [0u8; 32];
    for i in 0..32 {
        if i * 2 + 2 > pub_id_hex.len() { return false; }
        if let Ok(b) = u8::from_str_radix(&pub_id_hex[i * 2..i * 2 + 2], 16) {
            pub_key_bytes[i] = b;
        } else {
            return false;
        }
    }
    
    let mut sig_bytes = Vec::new();
    for i in 0..(signature_hex.len() / 2) {
        if i * 2 + 2 > signature_hex.len() { return false; }
        if let Ok(b) = u8::from_str_radix(&signature_hex[i * 2..i * 2 + 2], 16) {
            sig_bytes.push(b);
        } else {
            return false;
        }
    }
    
    let message = format!("{}:{}:{}", pub_id_hex, pseudo, timestamp);
    let public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, &pub_key_bytes);
    public_key.verify(message.as_bytes(), &sig_bytes).is_ok()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum Signal {
    Register { id: String, pseudo: String, timestamp: u64, signature: String },
    Unregister { id: String },
    Offer { sdp: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    ChatOffer { sdp: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    Answer { sdp: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    Ice { candidate: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    Ping { msg: String }
}

type Clients = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

#[tokio::main]
async fn main() {
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).await.expect("Impossible de lier le port");
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

async fn handle_connection(ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, clients: Clients) {
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
                    Signal::Register { id, pseudo, timestamp, signature } => {
                        if !verify_signature(&id, &pseudo, timestamp, &signature) {
                            continue;
                        }
                        let mut clients_guard = clients.lock().await;
                        if clients_guard.contains_key(&id) {
                            let _ = tx.send("ERROR:ALREADY_CONNECTED".to_string());
                        } else {
                            client_id = id.clone();
                            let (turn_user, turn_pass) = generate_turn_credentials(&client_id);
                            clients_guard.insert(id, tx.clone());
                            let _ = tx.send("SUCCESS:REGISTERED".to_string());
                            let _ = tx.send(format!("TURN_AUTH|{}|{}", turn_user, turn_pass));
                        }
                    },
                    Signal::Unregister { id } => {
                        if client_id == id {
                            let mut clients_guard = clients.lock().await;
                            clients_guard.remove(&id);
                            client_id.clear();
                        }
                    },
                    Signal::Offer { target_id, sender_id, .. } |
                    Signal::ChatOffer { target_id, sender_id, .. } |
                    Signal::Answer { target_id, sender_id, .. } |
                    Signal::Ice { target_id,  sender_id, .. } => {
                        if client_id.is_empty() || client_id != sender_id {
                            continue;
                        }
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

    if !client_id.is_empty() {
        clients.lock().await.remove(&client_id);
    }
    send_task.abort();
}
