use iced::Command;
use crate::ui::app::{KakolookiyamApp, AppState};
use crate::ui::messages::Message;
use crate::crypto;

impl KakolookiyamApp {
    pub(crate) fn handle_auth(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::GoToCreateAccount => { self.clear_auth_fields(); self.state = AppState::CreateAccount; }
            Message::GoToLogin => { self.clear_auth_fields(); self.state = AppState::Login; }
            Message::BackToWelcome => { self.clear_auth_fields(); self.state = AppState::Welcome; }

            Message::LockSession => {
                if let Some(vd) = &self.vault_data {
                    let id = crypto::derive_public_id(&vd.private_key);
                    let _ = self.tx_network.send(format!("LOGOUT:{}", id));
                }

                self.master_password = None;
                self.vault_data = None;
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.active_call = None;
                self.is_muted = false;
                self.selected_chat = None;
                self.chat_input.clear();
                self.chat_history.clear();
                self.new_group_input.clear();
                self.new_member_input.clear();

                self.clear_auth_fields();
                self.state = AppState::Login;
                self.status_message = "⏳ Prêt à appeler...".to_owned();
                self.idle_seconds = 0;
            }

            Message::ForceDisconnect(err_msg) => {
                let _ = self.handle_auth(Message::LockSession);
                self.auth_error = Some(err_msg);
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
                    let mut v_data = crypto::VaultData {
                        private_key: crypto::generate_secure_secret(),
                        pseudo: trimmed.to_string(),
                        contacts: std::collections::HashMap::new(),
                        groups: std::collections::HashMap::new(),
                        chat_history: std::collections::HashMap::new(),
                    };

                    match crypto::save_vault(&self.password_input, &mut v_data) {
                        Ok(_) => {
                            self.master_password = Some(self.password_input.clone());
                            self.auth_error = Some("⏳ Création réseau en cours...".to_string());

                            let id = crypto::derive_public_id(&v_data.private_key);
                            if let Some(tx) = self.tx_identity.take() {
                                let _ = tx.send((id.clone(), v_data.pseudo.clone()));
                            } else {
                                let _ = self.tx_network.send(format!("REGISTER:{}:{}", id, v_data.pseudo));
                            }
                            self.vault_data = Some(v_data);
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
                            self.auth_error = Some("⏳ Authentification réseau en cours...".to_string());

                            let id = crypto::derive_public_id(&v_data.private_key);
                            if let Some(tx) = self.tx_identity.take() {
                                let _ = tx.send((id.clone(), v_data.pseudo.clone()));
                            } else {
                                let _ = self.tx_network.send(format!("REGISTER:{}:{}", id, v_data.pseudo));
                            }
                            self.vault_data = Some(v_data);
                        }
                        Err(e) => self.auth_error = Some(e.to_string()),
                    }
                }
            }
            _ => {}
        }
        Command::none()
    }
}