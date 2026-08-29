#![allow(unused_mut)]

mod audio;
mod p2p;
mod ui;
mod crypto;

use iced::{Application, Font, Settings, Size}; // Ajout de Size
use std::borrow::Cow;
use ui::{KakolookiyamApp, Flags};
use tokio::sync::mpsc;

pub fn main() -> iced::Result {
    println!("🛡️ Lancement du client SecureP2P Kakolookiyam...");
    audio::detect_microphone();

    // Canaux de communication P2P <-> Interface
    let (tx_ui_to_p2p, rx_ui_to_p2p) = mpsc::unbounded_channel::<String>();
    let (tx_p2p_to_ui, rx_p2p_to_ui) = mpsc::unbounded_channel::<String>();

    // Canal d'attente pour l'identité cryptographique (ID + Pseudo)
    let (tx_identity, rx_identity) = std::sync::mpsc::channel::<(String, String)>();

    // Lancement des moteurs d'arrière-plan
    std::thread::spawn(move || {
        let (tx_mic, rx_mic) = tokio::sync::mpsc::channel::<Vec<u8>>(500);
        let (tx_speaker, rx_speaker) = std::sync::mpsc::channel::<Vec<i16>>();

        audio::start_hardware_audio(tx_mic, rx_speaker);

        // LE THREAD SE MET EN PAUSE ICI : Il attend que le coffre-fort soit ouvert !
        println!("⏳ Le moteur réseau est en attente du déverrouillage...");

        // On récupère l'ID et le Pseudo déchiffrés
        let (my_id_b64, my_pseudo) = rx_identity.recv().expect("L'interface s'est fermée avant le déverrouillage.");
        println!("🔓 Identité reçue ! Lancement du maillage P2P avec l'ID : {} (Pseudo: {})", my_id_b64, my_pseudo);

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // On passe le pseudo en 6ème argument au moteur WebRTC
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

    let mut settings = Settings::with_flags(flags);

    // --- OPTIMISATIONS QUALITY OF LIFE (QoL) ---
    settings.window = iced::window::Settings {
        size: Size::new(1024.0, 768.0), // Taille de lancement par défaut
        min_size: Some(Size::new(1024.0, 768.0)), // Taille minimum bloquée
        position: iced::window::Position::Centered, // Apparition au milieu de l'écran
        resizable: true, // Autorise le plein écran et l'étirement (Auto-stretch)
        ..Default::default()
    };

    settings.antialiasing = true; // Lissage indispensable pour la police Cinzel

    // --- INJECTION DE TOUTES LES DÉCLINAISONS DE LA POLICE ---
    settings.fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Cinzel-Regular.ttf")));
    settings.fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Cinzel-Medium.ttf")));
    settings.fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Cinzel-SemiBold.ttf")));
    settings.fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Cinzel-Bold.ttf")));
    settings.fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Cinzel-ExtraBold.ttf")));
    settings.fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Cinzel-Black.ttf")));

    // On force l'application à utiliser Cinzel Regular par défaut
    settings.default_font = Font {
        family: iced::font::Family::Name("Cinzel"),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    };

    KakolookiyamApp::run(settings)
}