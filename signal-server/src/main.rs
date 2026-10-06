use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{accept_hdr_async_with_config};
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use std::time::Duration;
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};

use hmac::{Hmac, Mac};
use sha1::Sha1;
use base64::Engine;

type HmacSha1 = Hmac<Sha1>;

fn generate_turn_credentials(secret: &str, user_id: &str) -> (String, String) {
    let unix_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let expiration = unix_time + 14400; // 4h
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

    let h = ring::digest::digest(&ring::digest::SHA256, b""); let hex: String = h.as_ref().iter().map(|b| format!("{:02x}", b)).collect(); let message = format!("KAKO-SIG-v2|Register|{}||{}|{}|{}", pub_id_hex, pseudo, timestamp, hex);
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

type Clients = Arc<Mutex<HashMap<String, (u64, mpsc::Sender<String>)>>>;
type ReplayCache = Arc<Mutex<HashMap<String, std::time::Instant>>>;

#[tokio::main]
async fn main() {
    let turn_secret = Arc::new(std::env::var("TURN_SECRET").expect("TURN_SECRET requis"));
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Impossible de lier le port");
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let replay_cache: ReplayCache = Arc::new(Mutex::new(HashMap::new()));
    let ip_tracker = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let sem = Arc::new(tokio::sync::Semaphore::new(1500));

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(_) => { tokio::time::sleep(Duration::from_millis(200)).await; continue; }
        };
        let Ok(permit) = sem.clone().try_acquire_owned() else { continue; };
        let clients_clone = clients.clone();
        let turn_secret_clone = turn_secret.clone();
        let cache_clone = replay_cache.clone();
        let ip_tracker_clone = ip_tracker.clone();

        tokio::spawn(async move {
            let _permit = permit;
            let mut cfg = WebSocketConfig::default();
            cfg.max_message_size = Some(64 * 1024);
            cfg.max_frame_size = Some(64 * 1024);
            
            let mut ip = String::new();
            let cb = |req: &Request, resp: Response| {
                if let Some(v) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
                    ip = v.split(',').next().unwrap_or("").trim().to_string();
                } else if let Some(v) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
                    ip = v.trim().to_string();
                }
                Ok(resp)
            };
            
            if let Ok(Ok(ws_stream)) = tokio::time::timeout(Duration::from_secs(10), accept_hdr_async_with_config(stream, cb, Some(cfg))).await {
                if !ip.is_empty() {
                    let mut tracker = ip_tracker_clone.lock().await;
                    let count = tracker.entry(ip.clone()).or_insert(0);
                    if *count >= 20 {
                        return; // Too many connections from this IP
                    }
                    *count += 1;
                }
                
                handle_connection(ws_stream, clients_clone, &turn_secret_clone, cache_clone).await;
                
                if !ip.is_empty() {
                    let mut tracker = ip_tracker_clone.lock().await;
                    if let Some(count) = tracker.get_mut(&ip) {
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            tracker.remove(&ip);
                        }
                    }
                }
            }
        });
    }
}

async fn handle_connection(ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, clients: Clients, turn_secret: &str, replay_cache: ReplayCache) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::channel::<String>(256);
    let mut client_id = String::new();
    static NEXT_CONN_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let conn_id = NEXT_CONN_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(tokio_tungstenite::tungstenite::protocol::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    let mut turn_refresh_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut win = std::time::Instant::now();
    let mut n = 0u32;

    let started = std::time::Instant::now();
    loop {
        let wait = if client_id.is_empty() {
            Duration::from_secs(15).saturating_sub(started.elapsed())
        } else {
            Duration::from_secs(90)
        };
        let next = tokio::time::timeout(wait, ws_receiver.next()).await;
        let Ok(Some(Ok(msg))) = next else { break };
        if let Ok(text) = msg.into_text() {
            if win.elapsed() > Duration::from_secs(10) { win = std::time::Instant::now(); n = 0; }
            n += 1; if n > 150 { continue; }

            if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                match signal {
                    Signal::Register { id, pseudo, timestamp, signature } => {
                        if !client_id.is_empty() { continue; }
                        if !verify_signature(&id, &pseudo, timestamp, &signature) {
                            let _ = tx.try_send(format!("ERROR:BAD_SIGNATURE:{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()));
                            continue;
                        }

                        {
                            let mut cache = replay_cache.lock().await;
                            cache.retain(|_, v| v.elapsed().as_secs() < 120);
                            if cache.contains_key(&signature) {
                                let _ = tx.try_send("ERROR:REPLAY".to_string());
                                continue;
                            }
                            cache.insert(signature.clone(), std::time::Instant::now());
                        }

                        let mut clients_guard = clients.lock().await;
                        client_id = id.clone();
                        let (turn_user, turn_pass) = generate_turn_credentials(turn_secret, &client_id);
                        if let Some((_old_conn_id, old_tx)) = clients_guard.insert(id, (conn_id, tx.clone())) {
                            let _ = old_tx.try_send("ERROR:ALREADY_CONNECTED".to_string());
                        }
                        let _ = tx.try_send("SUCCESS:REGISTERED".to_string());
                        let _ = tx.try_send(format!("TURN_AUTH|{}|{}", turn_user, turn_pass));

                        if let Some(t) = turn_refresh_task.take() { t.abort(); }
                        let tx2 = tx.clone();
                        let cid = client_id.clone();
                        let t_secret = turn_secret.to_string();
                        turn_refresh_task = Some(tokio::spawn(async move {
                            loop {
                                tokio::time::sleep(Duration::from_secs(3600)).await;
                                let (u, p) = generate_turn_credentials(&t_secret, &cid);
                                if tx2.try_send(format!("TURN_AUTH|{}|{}", u, p)).is_err() { break; }
                            }
                        }));
                    },
                    Signal::Unregister { id } => {
                        if client_id == id {
                            let mut clients_guard = clients.lock().await;
                            if matches!(clients_guard.get(&id), Some(&(cid, _)) if cid == conn_id) {
                                clients_guard.remove(&id);
                            }
                            client_id.clear();
                        }
                    },
                    Signal::Offer { target_id, sender_id, .. } => {
                        if client_id.is_empty() || client_id != sender_id { continue; }
                        let clients_guard = clients.lock().await;
                        if let Some((_, target_tx)) = clients_guard.get(&target_id) {
                            let _ = target_tx.try_send(text.to_string());
                        } else {
                            if let Some((_, sender_tx)) = clients_guard.get(&sender_id) {
                                let _ = sender_tx.try_send(format!("ERROR:NOT_FOUND:{}", target_id));
                            }
                        }
                    },
                    Signal::ChatOffer { target_id, sender_id, .. } |
                    Signal::Answer { target_id, sender_id, .. } |
                    Signal::Ice { target_id,  sender_id, .. } => {
                        if client_id.is_empty() || client_id != sender_id { continue; }
                        let clients_guard = clients.lock().await;
                        if let Some((_, target_tx)) = clients_guard.get(&target_id) {
                            let _ = target_tx.try_send(text.to_string());
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    if !client_id.is_empty() {
        let mut guard = clients.lock().await;
        if let Some(&(stored_conn_id, _)) = guard.get(&client_id) {
            if stored_conn_id == conn_id {
                guard.remove(&client_id);
            }
        }
    }
    if let Some(t) = turn_refresh_task { t.abort(); }
    send_task.abort();
}

