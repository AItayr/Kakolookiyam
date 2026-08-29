use iced::Command;
use crate::ui::app::{KakolookiyamApp, AppState};
use crate::ui::messages::Message;
use crate::crypto;

impl KakolookiyamApp {
    pub(crate) fn handle_network(&mut self, message: Message) -> Command<Message> {
        if let Message::NetworkEvent(msg) = message {
            self.idle_seconds = 0;

            if msg == "SUCCESS:REGISTERED" {
                self.state = AppState::Unlocked;
                self.clear_auth_fields();
                return Command::none();
            }

            if msg == "ERROR:ALREADY_CONNECTED" {
                return Command::perform(async {}, |_| { Message::ForceDisconnect("❌ Session déjà en cours.".to_string()) });
            }

            if msg == "ERROR:SERVER_OFFLINE" {
                return Command::perform(async {}, |_| { Message::ForceDisconnect("❌ Serveur injoignable.".to_string()) });
            }

            if msg.starts_with("FILE_RECV:") {
                let parts: Vec<&str> = msg.splitn(5, ':').collect();
                if parts.len() == 5 {
                    let sender_id = parts[1].trim().to_string();
                    let filename = parts[2].trim().to_string();
                    let key_b64 = parts[3].trim().to_string();
                    let path = parts[4].trim().to_string();

                    use base64::prelude::*;
                    let mut key_bytes = [0u8; 32];
                    if let Ok(decoded) = BASE64_STANDARD.decode(&key_b64) {
                        if decoded.len() == 32 { key_bytes.copy_from_slice(&decoded); }
                    }

                    let mut target_chat_id = sender_id.clone();
                    let mut display_filename = filename.clone();

                    if filename.contains('|') {
                        let f_parts: Vec<&str> = filename.splitn(2, '|').collect();
                        if f_parts.len() == 2 && f_parts[0].starts_with("grp_") {
                            target_chat_id = f_parts[0].trim().to_string();
                            display_filename = f_parts[1].trim().to_string();
                        }
                    }

                    let sender_pseudo = if let Some(vd) = &self.vault_data { vd.contacts.get(&sender_id).cloned().unwrap_or_else(|| "Inconnu".to_string()) } else { "Inconnu".to_string() };

                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                        let entry = crypto::MessageEntry {
                            author: sender_pseudo.clone(),
                            content: format!("📎 Fichier reçu : {}", display_filename),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                            is_media: true,
                            media_key: Some(key_bytes),
                            media_path: Some(path.clone()),
                        };
                        vd.chat_history.entry(target_chat_id.clone()).or_default().push(entry);
                        let _ = crypto::save_vault(pwd, vd);
                    }

                    let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_chat_id) || self.active_call.as_ref().map(|(id, _)| id) == Some(&target_chat_id);
                    if is_currently_viewed {
                        self.chat_history.push((sender_pseudo, format!("📎 Fichier reçu : {}", display_filename)));
                    }
                }
            }
            else if msg.starts_with("CHAT_RECV:") {
                let parts: Vec<&str> = msg.splitn(3, ':').collect();
                if parts.len() == 3 {
                    let sender_id = parts[1].trim().to_string();
                    let text = parts[2].to_string();

                    if text.starts_with("SYS:CALL_CONTEXT:") {
                        let sys_parts: Vec<&str> = text.splitn(3, ':').collect();
                        if sys_parts.len() == 3 {
                            let grp_id = sys_parts[2].trim().to_string();
                            if let Some(vd) = &self.vault_data {
                                if let Some(group) = vd.groups.get(&grp_id) {
                                    self.active_call = Some((grp_id.clone(), group.name.clone()));
                                    self.chat_history.clear();
                                    if let Some(history) = vd.chat_history.get(&grp_id) {
                                        for msg in history { self.chat_history.push((msg.author.clone(), msg.content.clone())); }
                                    }
                                    self.status_message = format!("📞 Conférence rejointe : {}", group.name);
                                }
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:GROUP_SYNC:") {
                        let sys_parts: Vec<&str> = text.splitn(5, ':').collect();
                        if sys_parts.len() == 5 {
                            let grp_id = sys_parts[2].trim().to_string();
                            let grp_name = sys_parts[3].trim().to_string();
                            let members: Vec<String> = sys_parts[4].split(',').map(|s| s.trim().to_string()).collect();

                            if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                                let is_new = !vd.groups.contains_key(&grp_id);
                                vd.groups.insert(grp_id.clone(), crate::crypto::GroupData { name: grp_name.clone(), members });
                                let _ = crate::crypto::save_vault(pwd, vd);
                                if is_new { self.status_message = format!("✅ Invité dans le serveur {} !", grp_name); }
                            }
                        }
                        return Command::none();
                    }

                    // --- FORCE DU TRIM POUR LA SUPPRESSION ---
                    if text.starts_with("SYS:GROUP_LEAVE:") {
                        let sys_parts: Vec<&str> = text.splitn(3, ':').collect();
                        if sys_parts.len() == 3 {
                            let grp_id = sys_parts[2].trim().to_string();
                            if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                                if let Some(group) = vd.groups.get_mut(&grp_id) {
                                    group.members.retain(|m| m.trim() != sender_id);
                                    let _ = crate::crypto::save_vault(pwd, vd);
                                    self.status_message = "🚪 Un membre a quitté le serveur.".to_string();
                                }
                            }
                        }
                        return Command::none();
                    }

                    let mut target_chat_id = sender_id.clone();
                    let mut display_text = text.clone();

                    if text.starts_with("SYS:GRP_MSG:") {
                        let sys_parts: Vec<&str> = text.splitn(4, ':').collect();
                        if sys_parts.len() == 4 {
                            target_chat_id = sys_parts[2].trim().to_string();
                            display_text = sys_parts[3].to_string();
                        }
                    }

                    let sender_pseudo = if let Some(vd) = &self.vault_data { vd.contacts.get(&sender_id).cloned().unwrap_or_else(|| "Inconnu".to_string()) } else { "Inconnu".to_string() };

                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                        let entry = crypto::MessageEntry {
                            author: sender_pseudo.clone(),
                            content: display_text.clone(),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                            is_media: false,
                            media_key: None,
                            media_path: None,
                        };
                        vd.chat_history.entry(target_chat_id.clone()).or_default().push(entry);
                        let _ = crypto::save_vault(pwd, vd);
                    }

                    let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_chat_id) || self.active_call.as_ref().map(|(id, _)| id) == Some(&target_chat_id);
                    if is_currently_viewed {
                        self.chat_history.push((sender_pseudo, display_text));
                    }
                }
            }
            else if msg.starts_with("CONTACT:") {
                let parts: Vec<&str> = msg.splitn(3, ':').collect();
                if parts.len() == 3 {
                    let c_id = parts[1].trim().to_string();
                    let c_pseudo = parts[2].trim().to_string();
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
                    let caller_id = parts[1].trim().to_string();
                    let sdp = parts[2].to_string();
                    let caller_pseudo = if let Some(vd) = &self.vault_data { vd.contacts.get(&caller_id).cloned().unwrap_or_else(|| "Inconnu".to_string()) } else { "Inconnu".to_string() };

                    self.incoming_call_timer = 0;
                    self.incoming_call = Some((caller_id, caller_pseudo, sdp));
                }
            }
            else if msg.starts_with("CALL_ACTIVE:") {
                let id = msg.trim_start_matches("CALL_ACTIVE:").trim().to_string();

                if let Some((active_id, _)) = &self.active_call {
                    if active_id.starts_with("grp_") {
                        let _ = self.tx_network.send(format!("CHAT_SEND:{}:SYS:CALL_CONTEXT:{}", id, active_id));
                    }
                } else {
                    let pseudo = if let Some(vd) = &self.vault_data { vd.contacts.get(&id).cloned().unwrap_or_else(|| "Ami".to_string()) } else { "Ami".to_string() };
                    self.active_call = Some((id.clone(), pseudo));

                    self.chat_input.clear();
                    self.chat_history.clear();
                    if let Some(vd) = &self.vault_data {
                        if let Some(history) = vd.chat_history.get(&id) {
                            for msg in history { self.chat_history.push((msg.author.clone(), msg.content.clone())); }
                        }
                    }
                }
            }
            else if msg.starts_with("CALL_ENDED:") {
                let id = msg.trim_start_matches("CALL_ENDED:").trim().to_string();
                let _ = self.tx_network.send(format!("HANGUP:{}", id));

                let mut should_end = false;
                if let Some((active_id, _)) = &self.active_call {
                    if active_id == &id || active_id.starts_with("grp_") {
                        should_end = true;
                    }
                }

                if should_end {
                    self.active_call = None;
                    self.is_muted = false;
                    let _ = self.tx_network.send("MUTE:off".to_string());
                    self.chat_input.clear();
                    self.chat_history.clear();
                    self.status_message = "L'interlocuteur a raccroché.".to_string();
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
        Command::none()
    }
}