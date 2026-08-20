use iced::{clipboard, Application, Command, Element, Subscription, Theme};
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
    pub tx_identity: std::sync::mpsc::Sender<String>, // Le canal pour débloquer le moteur P2P
}

pub struct KakolookiyamApp {
    pub(crate) state: AppState,

    // Champs d'authentification
    pub(crate) password_input: String,
    pub(crate) password_confirm_input: String,
    pub(crate) auth_error: Option<String>,

    // Champs de l'application
    pub(crate) my_local_id: String,
    pub(crate) peer_id_input: String,
    pub(crate) status_message: String,
    pub(crate) tx_network: UnboundedSender<String>,
    pub(crate) rx_network: Arc<Mutex<Option<UnboundedReceiver<String>>>>,
    pub(crate) tx_identity: Option<std::sync::mpsc::Sender<String>>,
}

impl Application for KakolookiyamApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        // Détection automatique : Si vault.kak existe -> Login, sinon -> Welcome
        let initial_state = if crypto::vault_exists() {
            AppState::Login
        } else {
            AppState::Welcome
        };

        (
            Self {
                state: initial_state,
                password_input: String::new(),
                password_confirm_input: String::new(),
                auth_error: None,

                my_local_id: String::new(), // Initialisé à vide (chargé au déverrouillage)
                peer_id_input: String::new(),
                status_message: "⏳ Prêt à appeler...".to_owned(),
                tx_network: flags.tx_network,
                rx_network: Arc::new(Mutex::new(Some(flags.rx_network))),
                tx_identity: Some(flags.tx_identity),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Kakolookiyam - Secure P2P")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // --- NAVIGATION ---
            Message::GoToCreateAccount => {
                self.clear_auth_fields();
                self.state = AppState::CreateAccount;
            }
            Message::GoToLogin => {
                self.clear_auth_fields();
                self.state = AppState::Login;
            }
            Message::BackToWelcome => {
                self.clear_auth_fields();
                self.state = AppState::Welcome;
            }

            // --- SAISIE ---
            Message::PasswordChanged(val) => {
                self.password_input = val;
                self.auth_error = None;
            }
            Message::PasswordConfirmChanged(val) => {
                self.password_confirm_input = val;
                self.auth_error = None;
            }

            // --- VALIDATION ET CRYPTOGRAPHIE ---
            Message::SubmitCreateAccount => {
                if self.password_input.is_empty() {
                    self.auth_error = Some("Le mot de passe ne peut pas être vide.".to_string());
                } else if self.password_input != self.password_confirm_input {
                    self.auth_error = Some("Les mots de passe ne correspondent pas.".to_string());
                } else {
                    // 1. Génération de clé sécurisée (OsRng)
                    let secret_key = crypto::generate_secure_secret();

                    // 2. Chiffrement et sauvegarde dans le coffre-fort
                    match crypto::create_vault(&self.password_input, &secret_key) {
                        Ok(_) => {
                            self.my_local_id = crypto::derive_public_id(&secret_key);
                            self.state = AppState::Unlocked;
                            self.clear_auth_fields();

                            // 3. Envoi de l'identité au thread P2P pour démarrer le réseau
                            if let Some(tx) = self.tx_identity.take() {
                                let _ = tx.send(self.my_local_id.clone());
                            }
                        }
                        Err(e) => {
                            self.auth_error = Some(e.to_string());
                        }
                    }
                }
            }
            Message::SubmitLogin => {
                if self.password_input.is_empty() {
                    self.auth_error = Some("Veuillez entrer un mot de passe.".to_string());
                } else {
                    // Déchiffrement du coffre-fort
                    match crypto::unlock_vault(&self.password_input) {
                        Ok(decrypted_key) => {
                            self.my_local_id = crypto::derive_public_id(&decrypted_key);
                            self.state = AppState::Unlocked;
                            self.clear_auth_fields();

                            // Envoi de l'identité au thread P2P pour démarrer le réseau
                            if let Some(tx) = self.tx_identity.take() {
                                let _ = tx.send(self.my_local_id.clone());
                            }
                        }
                        Err(e) => {
                            self.auth_error = Some(e.to_string());
                        }
                    }
                }
            }

            // --- LOGIQUE DES APPELS ---
            Message::PeerIdChanged(val) => self.peer_id_input = val,
            Message::ConnectClicked => {
                self.status_message = format!("🔗 Négociation avec : {}...", self.peer_id_input);
                let _ = self.tx_network.send(self.peer_id_input.clone());
            }
            Message::NetworkEvent(msg) => self.status_message = msg,
            Message::CopyIdClicked => {
                return clipboard::write(self.my_local_id.clone());
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
        iced::subscription::unfold(
            std::any::TypeId::of::<NetworkSub>(),
            self.rx_network.clone(),
            |rx_mutex| async move {
                let msg = {
                    let mut guard = rx_mutex.lock().await;
                    if let Some(rx) = guard.as_mut() {
                        rx.recv().await
                    } else {
                        None
                    }
                };
                match msg {
                    Some(text) => (Message::NetworkEvent(text), rx_mutex),
                    None => std::future::pending().await,
                }
            },
        )
    }
}