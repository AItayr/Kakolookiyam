#![allow(unused_mut)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod reduction;
mod audio;
mod network;
mod ui;
mod crypto;

use iced::{Font, Size};
use ui::{KakolookiyamApp, Flags};
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use iced::window::icon;

pub fn main() -> iced::Result {
    audio::detect_microphone();

    let (tx_ui_to_p2p, rx_ui_to_p2p) = mpsc::unbounded_channel::<String>();
    let (tx_p2p_to_ui, rx_p2p_to_ui) = mpsc::unbounded_channel::<String>();
    let (tx_identity, rx_identity) = std::sync::mpsc::channel::<(String, String, [u8; 32])>();
    let (tx_secrets, rx_secrets) = tokio::sync::mpsc::unbounded_channel::<[u8; 32]>();

    std::thread::spawn(move || {
        let (tx_mic, rx_mic) = tokio::sync::mpsc::channel::<Vec<u8>>(500);
        let (tx_speaker, rx_speaker) = std::sync::mpsc::channel::<(usize, Vec<i16>)>();

        audio::start_hardware_audio(tx_mic, rx_speaker);

        let (my_id_b64, my_pseudo, my_secret) = rx_identity.recv().expect("L'interface s'est fermee avant le deverrouillage.");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let _ = network::start_p2p(
                rx_mic,
                tx_speaker,
                rx_ui_to_p2p,
                tx_p2p_to_ui,
                my_id_b64,
                my_pseudo,
                my_secret,
                rx_secrets
            ).await;
        });
    });

    let flags = Flags {
        tx_network: tx_ui_to_p2p,
        rx_network: rx_p2p_to_ui,
        tx_identity,
        tx_secrets,
    };

    let default_font = Font {
        family: iced::font::Family::Name("Cinzel"),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    };

    let icon = icon::from_file_data(include_bytes!("../assets/images/Kakolookiyam_logo.png"), None).unwrap();

    // Make the boot function compatible with `BootFn` which returns `(State, Task<Message>)` or just `State`.
    // KakolookiyamApp::new returns `(KakolookiyamApp, iced::Task<Message>)`!
    
    // We pass `title` as a builder method on the returned Application!
    iced::application(
        {
        let flags_arc = Arc::new(Mutex::new(Some(flags)));
        move || KakolookiyamApp::new(flags_arc.lock().unwrap().take().unwrap())
    },
        KakolookiyamApp::update,
        KakolookiyamApp::view
    )
    .subscription(KakolookiyamApp::subscription)
    .theme(KakolookiyamApp::theme)
    .window(iced::window::Settings {
        icon: Some(icon),
        ..Default::default()
    })
    .window_size(Size::new(1024.0, 768.0))
    .centered()
    .antialiasing(true)
    .font(include_bytes!("../assets/fonts/Cinzel-Regular.ttf"))
    .font(include_bytes!("../assets/fonts/Cinzel-Medium.ttf"))
    .font(include_bytes!("../assets/fonts/Cinzel-SemiBold.ttf"))
    .font(include_bytes!("../assets/fonts/Cinzel-Bold.ttf"))
    .font(include_bytes!("../assets/fonts/Cinzel-ExtraBold.ttf"))
    .font(include_bytes!("../assets/fonts/Cinzel-Black.ttf"))
    .font(include_bytes!("../assets/fonts/ReemKufi-Regular.ttf"))
    .font(include_bytes!("../assets/fonts/ReemKufi-Medium.ttf"))
    .font(include_bytes!("../assets/fonts/ReemKufi-SemiBold.ttf"))
    .font(include_bytes!("../assets/fonts/ReemKufi-Bold.ttf"))
    .default_font(default_font)
    .title(KakolookiyamApp::title)
    .run()
}

// touch
