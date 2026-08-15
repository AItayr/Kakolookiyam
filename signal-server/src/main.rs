use std::io::Error;
use futures_util::{StreamExt, SinkExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio_tungstenite::accept_async;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Le serveur écoute sur le port 8080 en local pour l'instant
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(&addr).await?;
    println!("Serveur de signalisation 'aveugle' démarré sur : {}", addr);

    // Création d'un canal de diffusion (broadcast) d'une capacité de 16 messages.
    // Dès qu'un message est diffusé, il est oublié de la RAM.
    let (tx, _rx) = broadcast::channel(16);

    // Boucle infinie qui écoute les nouvelles connexions entrantes
    while let Ok((stream, _)) = listener.accept().await {
        let tx = tx.clone();
        tokio::spawn(accept_connection(stream, tx));
    }
    
    Ok(())
}

async fn accept_connection(stream: TcpStream, tx: broadcast::Sender<String>) {
    // On accepte la connexion WebSocket (SANS logger l'adresse IP entrante)
    if let Ok(ws_stream) = accept_async(stream).await {
        let (mut sender, mut receiver) = ws_stream.split();
        let mut rx = tx.subscribe();

        // Tâche 1 : Recevoir les messages du serveur et les envoyer au client
        let mut send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                // Utilisation de .into() pour la nouvelle version de Tungstenite
                if sender.send(tokio_tungstenite::tungstenite::Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        });

        // Tâche 2 : Recevoir les messages de ce client et les diffuser aveuglément aux autres
        let mut recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = receiver.next().await {
                if let Ok(text) = msg.into_text() {
                    // Le serveur sert uniquement de relai instantané. Aucun stockage !
                    // On convertit le Utf8Bytes en String classique pour notre canal
                    let _ = tx.send(text.to_string());
                }
            }
        });

        // Si l'une des tâches s'arrête (ex: déconnexion), on coupe l'autre proprement
        tokio::select! {
            _ = &mut send_task => recv_task.abort(),
            _ = &mut recv_task => send_task.abort(),
        };
    }
}