use iced::{time, Task as Command, Element, Event, Subscription, Theme};
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;

use crate::crypto;
use crate::ui::messages::Message;

pub enum AppState {
    Welcome,
    CreateAccount,
    Login,
    Unlocked,
}

pub struct Flags {
    pub tx_network: UnboundedSender<String>,
    pub rx_network: UnboundedReceiver<String>,
    pub tx_identity: std::sync::mpsc::Sender<(String, String, [u8; 32])>,
    pub tx_secrets: tokio::sync::mpsc::UnboundedSender<[u8; 32]>,
}

pub struct KakolookiyamApp {
    pub unread_counts: std::collections::HashMap<String, usize>,
    pub(crate) state: AppState,
    pub(crate) pseudo_input: String,
    pub(crate) password_input: secrecy::SecretString,
    pub(crate) password_confirm_input: secrecy::SecretString,
    pub(crate) auth_error: Option<String>,

    pub(crate) master_password: Option<secrecy::SecretString>,
    pub(crate) vault_data: Option<crypto::VaultData>,

    pub(crate) peer_id_input: String,
    pub(crate) status_message: String,
    pub(crate) tx_network: UnboundedSender<String>,
    pub(crate) rx_network: Arc<Mutex<Option<UnboundedReceiver<String>>>>,
    pub(crate) tx_identity: Option<std::sync::mpsc::Sender<(String, String, [u8; 32])>>,
    pub(crate) tx_secrets: tokio::sync::mpsc::UnboundedSender<[u8; 32]>,

    pub(crate) idle_seconds: u32,
    pub(crate) incoming_call: Option<(String, String, String, String)>,
    pub(crate) incoming_call_timer: u32,
    pub(crate) queued_group_offers: Vec<(String, String)>,
    pub(crate) active_call: Option<(String, String)>,
    pub(crate) is_muted: bool,

    pub(crate) selected_chat: Option<String>,
    pub(crate) chat_input: String,
    pub(crate) chat_history: Vec<(String, String)>,

    pub(crate) new_group_input: String,
    pub(crate) new_member_input: String,

    pub(crate) media_preview: Option<iced::widget::image::Handle>,

    pub(crate) current_theme: Theme,
    pub(crate) show_settings: bool,
    pub(crate) show_group_options: bool,
    pub(crate) needs_save: bool,

    // --- Variables d'interface (Paramètres) ---
    pub(crate) selected_mic: String,
    pub(crate) selected_speaker: String,
    pub(crate) available_mics: Vec<String>,
    pub(crate) available_speakers: Vec<String>,
    pub(crate) active_legal_tab: Option<String>,
    pub(crate) language: crate::ui::i18n::Language,
}

impl KakolookiyamApp {
                
    pub fn new(flags: Flags) -> (Self, Command<Message>) {
        let initial_state = if crypto::any_vault_exists() { AppState::Login } else { AppState::Welcome };
        (
            Self {
                unread_counts: std::collections::HashMap::new(),
                state: initial_state,
                pseudo_input: String::new(),
                password_input: secrecy::Secret::new(String::new()),
                password_confirm_input: secrecy::Secret::new(String::new()),
                auth_error: None,
                master_password: None,
                vault_data: None,
                peer_id_input: String::new(),
                status_message: "Prêt à appeler...".to_owned(),
                tx_network: flags.tx_network,
                rx_network: Arc::new(Mutex::new(Some(flags.rx_network))),
                tx_identity: Some(flags.tx_identity),
                tx_secrets: flags.tx_secrets,
                idle_seconds: 0,
                incoming_call: None,
                incoming_call_timer: 0,
            queued_group_offers: Vec::new(),
                active_call: None,
                is_muted: false,
                selected_chat: None,
                chat_input: String::new(),
                chat_history: Vec::new(),
                new_group_input: String::new(),
                new_member_input: String::new(),
                media_preview: None,
                current_theme: Theme::Dark,
                show_settings: false,
                show_group_options: false,
            needs_save: false,
                selected_mic: "Défaut".to_string(),
            available_mics: crate::audio::get_available_microphones(),
            available_speakers: crate::audio::get_available_speakers(),
                selected_speaker: "Défaut".to_string(),
                active_legal_tab: None,
                language: crate::ui::i18n::Language::Fr, // <-- INITIALISATION
            },
            Command::none(),
        )
    }

    pub fn title(&self) -> String { String::from("Kakolookiyam") }

    pub fn theme(&self) -> Theme {
        self.current_theme.clone()
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        if matches!(message, Message::LockSession | Message::ForceDisconnect(_)) {
            self.media_preview = None;
            self.show_settings = false;
            self.show_group_options = false;
            self.active_legal_tab = None;
        }

        if matches!(message, Message::DeselectChat | Message::SelectChat(_) | Message::DeleteGroup) {
            self.show_group_options = false;
        }

        match message {
            Message::MicSelected(mic) => { self.selected_mic = mic.clone(); crate::audio::set_microphone(mic); return Command::none(); }
            Message::SpeakerSelected(spk) => { self.selected_speaker = spk.clone(); crate::audio::set_speaker(spk); return Command::none(); }
            Message::ToggleLegal(tab) => {
                self.active_legal_tab = if self.active_legal_tab.as_deref() == Some(&tab) { None } else { Some(tab) };
                return Command::none();
            }

            Message::OpenSettings => { self.idle_seconds = 0; self.show_settings = true; return Command::none(); }
            Message::CloseSettings => { self.idle_seconds = 0; self.show_settings = false; return Command::none(); }
            Message::ToggleTheme => {
                self.idle_seconds = 0;
                self.current_theme = if self.current_theme == Theme::Dark { Theme::Light } else { Theme::Dark };
                return Command::none();
            }
            Message::ToggleLanguage => { // <-- INTERCEPTION DU BOUTON
                self.idle_seconds = 0;
                self.language = match self.language {
                    crate::ui::i18n::Language::Fr => crate::ui::i18n::Language::En,
                    crate::ui::i18n::Language::En => crate::ui::i18n::Language::Ar,
                    crate::ui::i18n::Language::Ar => crate::ui::i18n::Language::Fr,
                };
                return Command::none();
            }

            Message::OpenGroupOptions => { self.idle_seconds = 0; self.show_group_options = true; return Command::none(); }
            Message::CloseGroupOptions => { self.idle_seconds = 0; self.show_group_options = false; return Command::none(); }

            Message::GoToCreateAccount | Message::GoToLogin | Message::BackToWelcome |
            Message::LockSession | Message::ForceDisconnect(_) | Message::TickInactivity |
            Message::ResetInactivity | Message::PseudoChanged(_) | Message::PasswordChanged(_) |
            Message::PasswordConfirmChanged(_) | Message::SubmitCreateAccount | Message::SubmitLogin
            => self.handle_auth(message),

            Message::PeerIdChanged(_) | Message::ConnectClicked | Message::CallContact(_) |
            Message::AcceptCall(_, _, _) | Message::RejectCall(_) | Message::HangUpCall |
            Message::ToggleMute
            => self.handle_call(message),

            Message::SelectChat(_) | Message::DeselectChat | Message::ChatInputChanged(_) |
            Message::SendChatMessage | Message::CopyIdClicked | Message::CopyContactId(_) |
            Message::AcceptRequest(_) | Message::RejectRequest(_) | Message::BlockContact(_)
            => self.handle_chat(message),

            Message::OpenFileDialog | Message::FileSelected(_) | Message::FileRead(_) |
            Message::OpenMedia(_, _, _) | Message::MediaSaved(_) |
            Message::PreviewMedia(_, _) | Message::PreviewMediaLoaded(_) | Message::ClosePreview
            => self.handle_media(message),

            Message::NewGroupInputChanged(_) | Message::CreateGroup | Message::NewMemberInputChanged(_) |
            Message::AddMemberToGroup | Message::AddSpecificMemberToGroup(_) | Message::DeleteGroup
            => self.handle_group(message),

            Message::NetworkEvent(_)
            => self.handle_network(message),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.state {
            AppState::Welcome => self.view_welcome(),
            AppState::CreateAccount => self.view_create_account(),
            AppState::Login => self.view_login(),
            AppState::Unlocked => self.view_unlocked(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {

        let rx_network = self.rx_network.clone();


        #[derive(Clone)]
        struct RxData(std::sync::Arc<tokio::sync::Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<String>>>>);
        impl std::hash::Hash for RxData {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                std::sync::Arc::as_ptr(&self.0).hash(state);
            }
        }

        let network_subscription = iced::Subscription::run_with(
            RxData(rx_network),
            |rx_wrapper| {
                let rx_mutex = rx_wrapper.0.clone();
                
                iced::stream::channel::<Message>(100, |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
                    use futures_util::sink::SinkExt;
                    loop {
                        let msg = {
                            let mut guard = rx_mutex.lock().await;
                            if let Some(rx) = guard.as_mut() { 
                                rx.recv().await 
                            } else { 
                                None 
                            }
                        };
                        match msg {
                            Some(text) => {
                                let _ = output.send(Message::NetworkEvent(text)).await;
                            }
                            None => {
                                std::future::pending::<()>().await;
                            }
                        }
                    }
                })
            }
        );

        let timer_subscription = time::every(std::time::Duration::from_secs(1)).map(|_| Message::TickInactivity);
        let event_subscription = iced::event::listen_with(|event, _status, _window_id| {
            match event { Event::Keyboard(_) | Event::Mouse(_) => Some(Message::ResetInactivity), _ => None }
        });

        Subscription::batch(vec![network_subscription, timer_subscription, event_subscription])
    }
}
