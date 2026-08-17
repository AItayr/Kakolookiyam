use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};

// La structure de nos messages avec expéditeur et destinataire
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
enum Signal {
    Register { id: String },
    Offer { sdp: String, sender_id: String, target_id: String },
    Answer { sdp: String, sender_id: String, target_id: String },
    Ice { candidate: String, sender_id: String, target_id: String },
    Ping { msg: String }
}

// Dictionnaire sécurisé liant une Clé Publique à un "câble" de transmission WebSocket
type Clients = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<String>>>>;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await.expect("Impossible de lier le port");
    println!("📮 Serveur de signalisation (Facteur Privé) démarré sur : {}", addr);

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

    // Tâche d'arrière-plan pour envoyer les messages au client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(tokio_tungstenite::tungstenite::protocol::Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Écoute des messages entrants
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Ok(text) = msg.into_text() {
            if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                match signal {
                    // 1. Enregistrement de la clé publique du client
                    Signal::Register { id } => {
                        println!("📝 Nouvel utilisateur enregistré : {}", id);
                        client_id = id.clone();
                        clients.lock().await.insert(id, tx.clone());
                    },

                    // 2. Routage cryptographique privé
                    Signal::Offer { target_id, .. } | Signal::Answer { target_id, .. } | Signal::Ice { target_id, .. } => {
                        let clients_guard = clients.lock().await;
                        if let Some(target_tx) = clients_guard.get(&target_id) {
                            println!("📫 Routage secret d'un message vers : {}", target_id);
                            // CORRECTION ICI : Ajout de .to_string()
                            let _ = target_tx.send(text.to_string());
                        } else {
                            println!("⚠️ Destinataire introuvable ou hors-ligne : {}", target_id);
                        }
                    },
                    _ => {}
                }
            }
        }
    }

    // Si le client quitte l'application, on efface sa trace du serveur (Zéro-Trace)
    if !client_id.is_empty() {
        println!("❌ Utilisateur déconnecté : {}", client_id);
        clients.lock().await.remove(&client_id);
    }
    send_task.abort();
}