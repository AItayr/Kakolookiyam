use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};

#[tokio::main]
async fn main() {
    println!("Lancement du client SecureP2P...");

    // L'adresse locale de notre serveur de signalisation aveugle
    let url = "ws://127.0.0.1:8080";
    println!("Tentative de connexion au serveur sur {}...", url);

    // Tentative de connexion WebSocket
    match connect_async(url).await {
        Ok((mut ws_stream, _)) => {
            println!("Connecté avec succès au serveur de signalisation !");

            // Test d'envoi d'un message furtif
            let msg = "Présence d'un nouveau pair anonyme.";
            if ws_stream.send(Message::Text(msg.into())).await.is_ok() {
                println!("Message envoyé : '{}'", msg);
            }

            // Écoute en continu des messages entrants (pour les futures offres SDP)
            while let Some(Ok(response)) = ws_stream.next().await {
                if let Ok(text) = response.into_text() {
                    println!("Relai reçu du serveur : {}", text);
                }
            }
        },
        Err(e) => {
            println!("Impossible de se connecter au serveur : {}", e);
        }
    }
}