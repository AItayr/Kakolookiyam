#![allow(unused_mut)]

mod audio;
mod p2p;
mod interface;

use iced::{Application, Settings};
use interface::{KakolookiyamApp, Flags};
use tokio::sync::mpsc;

pub fn main() -> iced::Result {
    println!("🛡️ Lancement du client SecureP2P...");
    audio::detect_microphone();

    // CÂBLE 1 : UI -> Réseau (Pour envoyer l'ordre d'appel)
    let (tx_ui_to_p2p, rx_ui_to_p2p) = mpsc::unbounded_channel::<String>();

    // CÂBLE 2 : Réseau -> UI (Pour remonter l'état de l'appel)
    let (tx_p2p_to_ui, rx_p2p_to_ui) = mpsc::unbounded_channel::<String>();

    std::thread::spawn(move || {
        let (tx_mic, rx_mic) = tokio::sync::mpsc::channel::<Vec<u8>>(500);
        let (tx_speaker, rx_speaker) = std::sync::mpsc::channel::<Vec<i16>>();

        audio::start_hardware_audio(tx_mic, rx_speaker);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // On passe les deux extrémités au module réseau
            if let Err(e) = p2p::start_p2p(rx_mic, tx_speaker, rx_ui_to_p2p, tx_p2p_to_ui).await {
                eprintln!("❌ Erreur réseau P2P: {}", e);
            }
        });
    });

    // On donne les deux autres extrémités à l'interface
    let flags = Flags {
        tx_network: tx_ui_to_p2p,
        rx_network: rx_p2p_to_ui,
    };

    let mut settings = Settings::with_flags(flags);
    KakolookiyamApp::run(settings)
}