#![allow(unused_mut)]

mod audio;
mod p2p;
mod interface;
mod crypto; // NOUVEAU

use iced::{Application, Settings};
use interface::{KakolookiyamApp, Flags};
use tokio::sync::mpsc;

pub fn main() -> iced::Result {
    println!("🛡️ Lancement du client SecureP2P...");
    audio::detect_microphone();

   // GÉNÉRATION DE TON IDENTITÉ CRYPTOGRAPHIQUE
       let my_identity = crypto::generate_identity();
       let my_id_b64 = my_identity.public_key_b64.clone(); // NOUVEAU : On la clone pour le réseau
       println!("🔑 Mon ID Public (Base64) : {}", my_id_b64);

       let (tx_ui_to_p2p, rx_ui_to_p2p) = mpsc::unbounded_channel::<String>();
       let (tx_p2p_to_ui, rx_p2p_to_ui) = mpsc::unbounded_channel::<String>();

       std::thread::spawn(move || {
           let (tx_mic, rx_mic) = tokio::sync::mpsc::channel::<Vec<u8>>(500);
           let (tx_speaker, rx_speaker) = std::sync::mpsc::channel::<Vec<i16>>();

           audio::start_hardware_audio(tx_mic, rx_speaker);

           let rt = tokio::runtime::Runtime::new().unwrap();
           rt.block_on(async {
               // NOUVEAU : On passe notre my_id_b64 au moteur p2p !
               if let Err(e) = p2p::start_p2p(rx_mic, tx_speaker, rx_ui_to_p2p, tx_p2p_to_ui, my_id_b64).await {
                   eprintln!("❌ Erreur réseau P2P: {}", e);
               }
           });
       });

    // On passe l'ID cryptographique à l'interface
    let flags = Flags {
        tx_network: tx_ui_to_p2p,
        rx_network: rx_p2p_to_ui,
        my_local_id: my_identity.public_key_b64, // NOUVEAU
    };

    let mut settings = Settings::with_flags(flags);
    KakolookiyamApp::run(settings)
}