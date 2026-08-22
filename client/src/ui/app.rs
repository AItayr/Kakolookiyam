use iced::{clipboard, time, Application, Command, Element, Event, Subscription, Theme};
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

    // --- NOUVEAU : État du Chat P2P ---
    pub(crate) chat_input: String,
    pub(crate) chat_history: Vec<(String, String)>, // (Auteur, Message)
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
                chat_input: String::new(),
                chat_history: Vec::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String { String::from("Kakolookiyam - Secure P2P") }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::GoToCreateAccount => { self.clear_auth_fields(); self.state = AppState::CreateAccount; }
            Message::GoToLogin => { self.clear_auth_fields(); self.state = AppState::Login; }
            Message::BackToWelcome => { self.clear_auth_fields(); self.state = AppState::Welcome; }

            Message::LockSession => {
                self.master_password = None;
                self.vault_data = None;
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.active_call = None;
                self.is_muted = false;
                // NOUVEAU : Destruction sécurisée de la mémoire du chat
                self.chat_input.clear();
                self.chat_history.clear();

                self.clear_auth_fields();
                self.state = AppState::Login;
                self.status_message = "⏳ Prêt à appeler...".to_owned();
                self.idle_seconds = 0;
            }
            Message::TickInactivity => {
                if self.incoming_call.is_some() {
                    self.incoming_call_timer += 1;
                    if self.incoming_call_timer >= 15 {
                        if let Some((id, _, _)) = self.incoming_call.take() {
                            let _ = self.tx_network.send(format!("REJECT:{}", id));
                            self.status_message = "Appel manqué.".to_string();
                        }
                        self.incoming_call_timer = 0;
                    }
                }
                else if matches!(self.state, AppState::Unlocked) {
                    self.idle_seconds += 1;
                    if self.idle_seconds >= 300 { return Command::perform(async {}, |_| Message::LockSession); }
                } else {
                    self.idle_seconds = 0;
                }
            }
            Message::ResetInactivity => { self.idle_seconds = 0; }
            Message::PseudoChanged(val) => { self.idle_seconds = 0; self.pseudo_input = val; self.auth_error = None; }
            Message::PasswordChanged(val) => { self.idle_seconds = 0; self.password_input = val; self.auth_error = None; }
            Message::PasswordConfirmChanged(val) => { self.idle_seconds = 0; self.password_confirm_input = val; self.auth_error = None; }

            Message::SubmitCreateAccount => {
                self.idle_seconds = 0;
                let trimmed = self.pseudo_input.trim();
                let potential_file = crypto::get_vault_file(trimmed);

                if trimmed.is_empty() { self.auth_error = Some("Veuillez choisir un pseudo.".into()); }
                else if std::path::Path::new(&potential_file).exists() { self.auth_error = Some("Ce profil existe déjà sur cet ordinateur.".into()); }
                else if self.password_input != self.password_confirm_input { self.auth_error = Some("Mots de passe distincts.".into()); }
                else {
                    let v_data = crypto::VaultData {
                        private_key: crypto::generate_secure_secret(),
                        pseudo: trimmed.to_string(),
                        contacts: std::collections::HashMap::new(),
                    };
                    match crypto::save_vault(&self.password_input, &v_data) {
                        Ok(_) => {
                            self.master_password = Some(self.password_input.clone());
                            self.vault_data = Some(v_data.clone());
                            self.state = AppState::Unlocked;
                            self.clear_auth_fields();
                            if let Some(tx) = self.tx_identity.take() {
                                let id = crypto::derive_public_id(&v_data.private_key);
                                let _ = tx.send((id, v_data.pseudo));
                            }
                        }
                        Err(e) => self.auth_error = Some(e.to_string()),
                    }
                }
            }
            Message::SubmitLogin => {
                self.idle_seconds = 0;
                let trimmed = self.pseudo_input.trim();

                if trimmed.is_empty() {
                    self.auth_error = Some("Veuillez entrer votre pseudo.".to_string());
                } else if self.password_input.is_empty() {
                    self.auth_error = Some("Veuillez entrer un mot de passe.".to_string());
                } else {
                    match crypto::unlock_vault(trimmed, &self.password_input) {
                        Ok(v_data) => {
                            self.master_password = Some(self.password_input.clone());
                            self.vault_data = Some(v_data.clone());
                            self.state = AppState::Unlocked;
                            self.clear_auth_fields();
                            if let Some(tx) = self.tx_identity.take() {
                                let id = crypto::derive_public_id(&v_data.private_key);
                                let _ = tx.send((id, v_data.pseudo));
                            }
                        }
                        Err(e) => self.auth_error = Some(e.to_string()),
                    }
                }
            }

            Message::PeerIdChanged(val) => { self.idle_seconds = 0; self.peer_id_input = val; }
            Message::ConnectClicked => {
                self.idle_seconds = 0;
                self.status_message = format!("🔗 En attente de l'interlocuteur...");
                let _ = self.tx_network.send(format!("CALL:{}", self.peer_id_input));
            }
            Message::CallContact(id) => {
                self.idle_seconds = 0;
                self.peer_id_input = id.clone();
                self.status_message = format!("🔗 En attente de l'interlocuteur...");
                let _ = self.tx_network.send(format!("CALL:{}", id));
            }
            Message::AcceptCall(id, sdp) => {
                self.idle_seconds = 0;
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.status_message = format!("🔗 Connexion sécurisée en cours...");
                let _ = self.tx_network.send(format!("ACCEPT:{}:{}", id, sdp));
            }
            Message::RejectCall(id) => {
                self.idle_seconds = 0;
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.status_message = format!("❌ Appel rejeté.");
                let _ = self.tx_network.send(format!("REJECT:{}", id));
            }
            Message::HangUpCall => {
                self.idle_seconds = 0;
                if let Some((id, _)) = self.active_call.take() {
                    let _ = self.tx_network.send(format!("HANGUP:{}", id));
                    self.status_message = "Appel terminé.".to_string();
                }
                self.is_muted = false;
                // NOUVEAU : On vide le chat quand on raccroche
                self.chat_input.clear();
                self.chat_history.clear();
                let _ = self.tx_network.send("MUTE:off".to_string());
            }
            Message::ToggleMute => {
                self.idle_seconds = 0;
                self.is_muted = !self.is_muted;
                if self.is_muted { let _ = self.tx_network.send("MUTE:on".to_string()); }
                else { let _ = self.tx_network.send("MUTE:off".to_string()); }
            }

            // --- NOUVEAU : Logique de la saisie de texte ---
            Message::ChatInputChanged(val) => {
                self.idle_seconds = 0;
                self.chat_input = val;
            }
            Message::SendChatMessage => {
                self.idle_seconds = 0;
                let text = self.chat_input.trim().to_string();
                if !text.is_empty() {
                    if let Some((target_id, _)) = &self.active_call {
                        // Envoi de la commande structurée au moteur réseau
                        let _ = self.tx_network.send(format!("CHAT_SEND:{}:{}", target_id, text));
                        // Ajout visuel local
                        self.chat_history.push(("Moi".to_string(), text));
                        self.chat_input.clear();
                    }
                }
            }

            Message::NetworkEvent(msg) => {
                self.idle_seconds = 0;

                // --- NOUVEAU : Interception d'un message entrant P2P ---
                if msg.starts_with("CHAT_RECV:") {
                    let parts: Vec<&str> = msg.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let sender_id = parts[1].to_string();
                        let text = parts[2].to_string();

                        let sender_pseudo = if let Some(vd) = &self.vault_data {
                            vd.contacts.get(&sender_id).cloned().unwrap_or_else(|| "Inconnu".to_string())
                        } else { "Inconnu".to_string() };

                        self.chat_history.push((sender_pseudo, text));
                    }
                }
                else if msg.starts_with("CONTACT:") {
                    let parts: Vec<&str> = msg.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let c_id = parts[1].to_string();
                        let c_pseudo = parts[2].to_string();
                        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                            if !vd.contacts.contains_key(&c_id) {
                                vd.contacts.insert(c_id, c_pseudo);
                                let _ = crypto::save_vault(pwd, vd);
                            }
                        }
                    }
                }
                else if msg.starts_with("INCOMING_CALL:") {
                    let parts: Vec<&str> = msg.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let caller_id = parts[1].to_string();
                        let sdp = parts[2].to_string();
                        let caller_pseudo = if let Some(vd) = &self.vault_data {
                            vd.contacts.get(&caller_id).cloned().unwrap_or_else(|| "Inconnu".to_string())
                        } else { "Inconnu".to_string() };

                        self.incoming_call_timer = 0;
                        self.incoming_call = Some((caller_id, caller_pseudo, sdp));
                    }
                }
                else if msg.starts_with("CALL_ACTIVE:") {
                    let id = msg.trim_start_matches("CALL_ACTIVE:").to_string();
                    let pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts.get(&id).cloned().unwrap_or_else(|| "Ami".to_string())
                    } else { "Ami".to_string() };
                    self.active_call = Some((id, pseudo));

                    // On purge le chat pour être sûr d'avoir un écran propre à chaque nouvel appel
                    self.chat_input.clear();
                    self.chat_history.clear();
                }
                else if msg.starts_with("CALL_ENDED:") {
                    let id = msg.trim_start_matches("CALL_ENDED:").to_string();
                    if let Some((active_id, _)) = &self.active_call {
                        if active_id == &id {
                            self.active_call = None;
                            self.is_muted = false;
                            let _ = self.tx_network.send("MUTE:off".to_string());
                            self.chat_input.clear();
                            self.chat_history.clear();
                            self.status_message = "L'interlocuteur a raccroché.".to_string();
                        }
                    }
                    if let Some((inc_id, _, _)) = &self.incoming_call {
                        if inc_id == &id {
                            self.incoming_call = None;
                            self.incoming_call_timer = 0;
                            self.status_message = "L'appelant a raccroché.".to_string();
                        }
                    }
                }
                else { self.status_message = msg; }
            }
            Message::CopyIdClicked => {
                self.idle_seconds = 0;
                if let Some(vd) = &self.vault_data {
                    return clipboard::write(crypto::derive_public_id(&vd.private_key));
                }
            }
        }
        Command::none()
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