use secrecy::ExposeSecret;
use iced::Task as Command;
use crate::ui::app::{KakolookiyamApp, AppState};
use crate::ui::messages::Message;
use zeroize::Zeroize;
use crate::crypto;

impl KakolookiyamApp {
    pub(crate) fn handle_auth(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::GoToCreateAccount => { self.clear_auth_fields(); self.state = AppState::CreateAccount; }
            Message::GoToLogin => { self.clear_auth_fields(); self.state = AppState::Login; }
            Message::BackToWelcome => { self.clear_auth_fields(); self.state = AppState::Welcome; }

            Message::LockSession => {
                if let Some(mut vd) = self.vault_data.take() {
                    let id = crypto::derive_public_id(&vd.private_key);
                    let _ = self.tx_network.send(format!("LOGOUT:{}", id));
                    // [MITIGATION] Zero-Trace RAM: Wiping skipped nested HashMaps in VaultData
                    vd.zeroize_deep();
                }

                self.master_password = None;
                self.vault_data = None;
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.active_call = None;
                self.is_muted = false;
                self.selected_chat = None;
                self.chat_input.zeroize();
                self.chat_input.clear();
                
                // [MITIGATION] Zero-Trace RAM: Wipe active chat UI history strings
                for (author, content) in self.chat_history.iter_mut() {
                    author.zeroize();
                    content.zeroize();
                }
                self.chat_history.clear();
                self.new_group_input.zeroize();
                self.new_group_input.clear();
                self.new_member_input.zeroize();
                self.new_member_input.clear();

                self.clear_auth_fields();
                crate::sound::SOUND_MANAGER.lock().unwrap().play_main_theme();
                self.state = AppState::Login;
                        self.status_message = crate::ui::i18n::t(&self.language, "ready_to_call");
                self.idle_seconds = 0;
            }
            Message::ForceDisconnect(err_msg) => {
                let _ = self.handle_auth(Message::LockSession);
                self.auth_error = Some(err_msg);
            }
            Message::TickInactivity => {
                self.heartbeat_counter = self.heartbeat_counter.wrapping_add(1);
                
                let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();

                // 1) Cleanup old presences (15 secs window)
                for (_, presences) in self.group_call_presences.iter_mut() {
                    presences.retain(|_, last_seen| now.saturating_sub(*last_seen) < 15);
                }
                self.group_call_presences.retain(|_, presences| !presences.is_empty());

                // 2) Send our heartbeat if we are in a group call (every 5 seconds)
                if self.heartbeat_counter % 5 == 0 {
                    if let Some((active_id, _)) = &self.active_call {
                        if active_id.starts_with("grp_") {
                            if let Some(vd) = &self.vault_data {
                                if let Some(group) = vd.groups.get(active_id) {
                                    let my_id = crate::crypto::derive_public_id(&vd.private_key);
                                    let tx = self.tx_network.clone();
                                    for member_id in &group.members {
                                        if member_id != &my_id {
                                            let _ = tx.send(format!("CHAT_SEND_IF_OPEN:{}:SYS:GRP_HEARTBEAT:{}", member_id, active_id));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if self.needs_save {
                    self.needs_save = false;
                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                        let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                    }
                }
                if self.incoming_call.is_some() {
                    self.incoming_call_timer += 1;
                    if self.incoming_call_timer >= 15 {
                        if let Some((id, _, _, _)) = self.incoming_call.take() {
                            let _ = self.tx_network.send(format!("REJECT:{}", id));
                            self.status_message = crate::ui::i18n::t(&self.language, "status_call_missed");
                            crate::sound::SOUND_MANAGER.lock().unwrap().stop_incoming_call();
                        }
                        self.incoming_call_timer = 0;
                    }
                } else if matches!(self.state, AppState::Unlocked) {
                    self.idle_seconds += 1;
                    if self.idle_seconds >= 300 {
                        return Command::perform(async {}, |_| Message::LockSession);
                    }
                    
                    
                } else {
                    self.idle_seconds = 0;
                }
            }
            Message::ResetInactivity => { self.idle_seconds = 0; }
            Message::PseudoChanged(val) => { self.idle_seconds = 0; self.pseudo_input = val; self.auth_error = None; }
            Message::PasswordChanged(val) => { self.idle_seconds = 0; self.password_input = secrecy::Secret::new(val); self.auth_error = None; }
            Message::PasswordConfirmChanged(val) => { self.idle_seconds = 0; self.password_confirm_input = secrecy::Secret::new(val); self.auth_error = None; }

            Message::SubmitCreateAccount => {
                self.idle_seconds = 0;
                let trimmed = self.pseudo_input.trim();
                let potential_file = crypto::get_vault_file(trimmed);

                if trimmed.is_empty() {
                    self.auth_error = Some(crate::ui::i18n::t(&self.language, "err_choose_pseudo"));
                } else if std::path::Path::new(&potential_file).exists() {
                    self.auth_error = Some(crate::ui::i18n::t(&self.language, "err_profile_exists")); //  déjà sur cet ordinateur.".into());
                } else if self.password_input.expose_secret() != self.password_confirm_input.expose_secret() {
                    self.auth_error = Some(crate::ui::i18n::t(&self.language, "err_passwords_match"));
                } else {
                    let mut v_data = crypto::VaultData {
                        private_key: crypto::generate_secure_secret(),
                        pseudo: trimmed.to_string(),
                        contacts: std::collections::HashMap::new(),
                        pending_requests: std::collections::HashMap::new(),
                        groups: std::collections::HashMap::new(),
                        chat_history: std::collections::HashMap::new(),
                        blocked_ids: std::collections::HashSet::new(),
                        tombstones: std::collections::HashSet::new(),
                        session_key: None,
                        session_salt: None,
                    };

                    match crypto::save_vault(self.password_input.expose_secret(), &mut v_data) {
                        Ok(_) => {
                            self.master_password = Some(secrecy::Secret::new(self.password_input.expose_secret().clone()));
                            self.auth_error = None;

                            let id = crypto::derive_public_id(&v_data.private_key);

                            if let Some(tx) = self.tx_identity.take() {
                                let _ = tx.send((id.clone(), v_data.pseudo.clone(), v_data.private_key));
                                let mut sync_str = String::from("CONTACTS_SYNC");
                                for (c_id, _) in &v_data.contacts { sync_str.push_str(":"); sync_str.push_str(c_id); }
                                let _ = self.tx_network.send(sync_str);
                            } else {
                                let _ = self.tx_network.send(format!("REGISTER:{}:{}", id, v_data.pseudo));
                                let _ = self.tx_secrets.send(v_data.private_key);
                                let mut sync_str = String::from("CONTACTS_SYNC");
                                for (c_id, _) in &v_data.contacts { sync_str.push_str(":"); sync_str.push_str(c_id); }
                                let _ = self.tx_network.send(sync_str);
                            }
                            self.vault_data = Some(v_data);
                        }
                        Err(e) => { crate::sound::SOUND_MANAGER.lock().unwrap().play_error(); self.auth_error = Some(e.to_string()); },
                    }
                }
            }
            Message::SubmitLogin => {
                self.idle_seconds = 0;
                let trimmed = self.pseudo_input.trim();

                if trimmed.is_empty() {
                    self.auth_error = Some(crate::ui::i18n::t(&self.language, "err_enter_pseudo"));
                } else if self.password_input.expose_secret().is_empty() {
                    self.auth_error = Some(crate::ui::i18n::t(&self.language, "err_enter_password"));
                } else {
                    match crypto::unlock_vault(trimmed, self.password_input.expose_secret()) {
                        Ok(v_data) => {
                            self.master_password = Some(secrecy::Secret::new(self.password_input.expose_secret().clone()));
                            self.auth_error = None;

                            let id = crypto::derive_public_id(&v_data.private_key);

                            if let Some(tx) = self.tx_identity.take() {
                                let _ = tx.send((id.clone(), v_data.pseudo.clone(), v_data.private_key));
                                let mut sync_str = String::from("CONTACTS_SYNC");
                                for (c_id, _) in &v_data.contacts { sync_str.push_str(":"); sync_str.push_str(c_id); }
                                let _ = self.tx_network.send(sync_str);
                            } else {
                                let _ = self.tx_network.send(format!("REGISTER:{}:{}", id, v_data.pseudo));
                                let _ = self.tx_secrets.send(v_data.private_key);
                                let mut sync_str = String::from("CONTACTS_SYNC");
                                for (c_id, _) in &v_data.contacts { sync_str.push_str(":"); sync_str.push_str(c_id); }
                                let _ = self.tx_network.send(sync_str);
                            }
                            self.vault_data = Some(v_data);
                        }
                        Err(e) => { crate::sound::SOUND_MANAGER.lock().unwrap().play_error(); self.auth_error = Some(e.to_string()); },
                    }
                }
            }
            _ => {}
        }
        Command::none()
    }
}
