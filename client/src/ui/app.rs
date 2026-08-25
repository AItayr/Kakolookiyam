use iced::{time, Application, Command, Element, Event, Subscription, Theme};
use std::sync::Arc;
use std::time::Duration;
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
    pub tx_identity: std::sync::mpsc::Sender<(String, String)>,
}

pub struct KakolookiyamApp {
    pub(crate) state: AppState,
    pub(crate) pseudo_input: String,
    pub(crate) password_input: String,
    pub(crate) password_confirm_input: String,
    pub(crate) auth_error: Option<String>,

    pub(crate) master_password: Option<String>,
    pub(crate) vault_data: Option<crypto::VaultData>,

    pub(crate) peer_id_input: String,
    pub(crate) status_message: String,
    pub(crate) tx_network: UnboundedSender<String>,
    pub(crate) rx_network: Arc<Mutex<Option<UnboundedReceiver<String>>>>,
    pub(crate) tx_identity: Option<std::sync::mpsc::Sender<(String, String)>>,

    pub(crate) idle_seconds: u32,
    pub(crate) incoming_call: Option<(String, String, String)>,
    pub(crate) incoming_call_timer: u32,
    pub(crate) active_call: Option<(String, String)>,
    pub(crate) is_muted: bool,

    pub(crate) selected_chat: Option<String>,
    pub(crate) chat_input: String,
    pub(crate) chat_history: Vec<(String, String)>,

    pub(crate) new_group_input: String,
    pub(crate) new_member_input: String,
}

impl Application for KakolookiyamApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let initial_state = if crypto::any_vault_exists() { AppState::Login } else { AppState::Welcome };
        (
            Self {
                state: initial_state,
                pseudo_input: String::new(),
                password_input: String::new(),
                password_confirm_input: String::new(),
                auth_error: None,
                master_password: None,
                vault_data: None,
                peer_id_input: String::new(),
                status_message: "⏳ Prêt à appeler...".to_owned(),
                tx_network: flags.tx_network,
                rx_network: Arc::new(Mutex::new(Some(flags.rx_network))),
                tx_identity: Some(flags.tx_identity),
                idle_seconds: 0,
                incoming_call: None,
                incoming_call_timer: 0,
                active_call: None,
                is_muted: false,
                selected_chat: None,
                chat_input: String::new(),
                chat_history: Vec::new(),
                new_group_input: String::new(),
                new_member_input: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String { String::from("Kakolookiyam - Secure P2P") }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::GoToCreateAccount | Message::GoToLogin | Message::BackToWelcome |
            Message::LockSession | Message::ForceDisconnect(_) | Message::TickInactivity |
            Message::ResetInactivity | Message::PseudoChanged(_) | Message::PasswordChanged(_) |
            Message::PasswordConfirmChanged(_) | Message::SubmitCreateAccount | Message::SubmitLogin
            => self.handle_auth(message),

            // Routage propre, sans les fantômes !
            Message::PeerIdChanged(_) | Message::ConnectClicked | Message::CallContact(_) |
            Message::AcceptCall(_, _) | Message::RejectCall(_) | Message::HangUpCall |
            Message::ToggleMute
            => self.handle_call(message),

            Message::SelectChat(_) | Message::DeselectChat | Message::ChatInputChanged(_) |
            Message::SendChatMessage | Message::CopyIdClicked | Message::CopyContactId(_)
            => self.handle_chat(message),

            Message::OpenFileDialog | Message::FileSelected(_) | Message::FileRead(_) |
            Message::OpenMedia(_, _, _) | Message::MediaSaved(_)
            => self.handle_media(message),

            Message::NewGroupInputChanged(_) | Message::CreateGroup | Message::NewMemberInputChanged(_) |
            Message::AddMemberToGroup | Message::AddSpecificMemberToGroup(_) | Message::DeleteGroup
            => self.handle_group(message),

            Message::NetworkEvent(_)
            => self.handle_network(message),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self.state {
            AppState::Welcome => self.view_welcome(),
            AppState::CreateAccount => self.view_create_account(),
            AppState::Login => self.view_login(),
            AppState::Unlocked => self.view_unlocked(),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        struct NetworkSub;
        let network_subscription = iced::subscription::unfold(
            std::any::TypeId::of::<NetworkSub>(),
            self.rx_network.clone(),
            |rx_mutex| async move {
                let msg = {
                    let mut guard = rx_mutex.lock().await;
                    if let Some(rx) = guard.as_mut() { rx.recv().await } else { None }
                };
                match msg { Some(text) => (Message::NetworkEvent(text), rx_mutex), None => std::future::pending().await }
            },
        );

        let timer_subscription = time::every(Duration::from_secs(1)).map(|_| Message::TickInactivity);
        let event_subscription = iced::event::listen_with(|event, _status| {
            match event { Event::Keyboard(_) | Event::Mouse(_) => Some(Message::ResetInactivity), _ => None }
        });

        Subscription::batch(vec![network_subscription, timer_subscription, event_subscription])
    }
}