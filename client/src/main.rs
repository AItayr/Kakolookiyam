#![allow(unused_mut)]

mod audio;
mod p2p;
mod ui;
mod crypto;

use iced::{Application, Settings};
use ui::{KakolookiyamApp, Flags};
use tokio::sync::mpsc;

pub fn main() -> iced::Result {
    println!("🛡️ Lancement du client SecureP2P Kakolookiyam...");
    audio::detect_microphone();

    // Canaux de communication P2P <-> Interface
    let (tx_ui_to_p2p, rx_ui_to_p2p) = mpsc::unbounded_channel::<String>();
    let (tx_p2p_to_ui, rx_p2p_to_ui) = mpsc::unbounded_channel::<String>();

    // NOUVEAU : Canal d'attente pour l'identité cryptographique (ID + Pseudo)
    let (tx_identity, rx_identity) = std::sync::mpsc::channel::<(String, String)>();

    // Lancement des moteurs d'arrière-plan
    std::thread::spawn(move || {
        let (tx_mic, rx_mic) = tokio::sync::mpsc::channel::<Vec<u8>>(500);
        let (tx_speaker, rx_speaker) = std::sync::mpsc::channel::<Vec<i16>>();

        audio::start_hardware_audio(tx_mic, rx_speaker);

        // LE THREAD SE MET EN PAUSE ICI : Il attend que le coffre-fort soit ouvert !
        println!("⏳ Le moteur réseau est en attente du déverrouillage...");

        // NOUVEAU : On récupère l'ID et le Pseudo déchiffrés
        let (my_id_b64, my_pseudo) = rx_identity.recv().expect("L'interface s'est fermée avant le déverrouillage.");
        println!("🔓 Identité reçue ! Lancement du maillage P2P avec l'ID : {} (Pseudo: {})", my_id_b64, my_pseudo);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // NOUVEAU : On passe le pseudo en 6ème argument au moteur WebRTC
            if let Err(e) = p2p::start_p2p(rx_mic, tx_speaker, rx_ui_to_p2p, tx_p2p_to_ui, my_id_b64, my_pseudo).await {
                eprintln!("❌ Erreur réseau P2P: {}", e);
            }
        });
    });

    // Lancement de l'interface graphique (elle démarre vide et verrouillée)
    let flags = Flags {
        tx_network: tx_ui_to_p2p,
        rx_network: rx_p2p_to_ui,
        tx_identity, // On donne le transmetteur à l'interface
    };

    let settings = Settings::with_flags(flags);
    KakolookiyamApp::run(settings)
}