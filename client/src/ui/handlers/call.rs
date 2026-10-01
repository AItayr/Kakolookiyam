use secrecy::ExposeSecret;
use iced::Task as Command;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::sound::SOUND_MANAGER;

impl KakolookiyamApp {
    pub(crate) fn handle_call(&mut self, message: Message) -> Command<Message> {
        self.idle_seconds = 0;

        match message {
            Message::PeerIdChanged(val) => {
                self.peer_id_input = val;
            }
Message::ConnectClicked => {
                if let Some(vd) = &self.vault_data {
                    let my_id = crate::crypto::derive_public_id(&vd.private_key);
                    if self.peer_id_input.trim().to_lowercase() == my_id.to_lowercase() {
                        self.status_message = crate::ui::i18n::t(&self.language, "error_self_call");
                        return Command::none(); // Blocage auto-appel via ajout manuel
                    }
                }

                if self.active_call.is_some() {
                    let _ = self.handle_call(Message::HangUpCall);
                }

                if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                    if !self.peer_id_input.trim().is_empty() {
                        let id = self.peer_id_input.trim().to_string();
                        if !vd.contacts.contains_key(&id) && !vd.pending_requests.contains_key(&id) {
                            vd.contacts.insert(id.clone(), "Ajout manuel (Inconnu)".to_string());
                            let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                        }
                    }
                }
                        self.status_message = crate::ui::i18n::t(&self.language, "status_call_waiting");
                SOUND_MANAGER.lock().unwrap().start_outgoing_call();
                let _ = self.tx_network.send(format!("CALL:{}", self.peer_id_input));
            }
            Message::CallContact(target_id) => {

                if let Some(vd) = &self.vault_data {
                    let my_id = crate::crypto::derive_public_id(&vd.private_key);
                    if target_id.to_lowercase() == my_id.to_lowercase() {
                        self.status_message = crate::ui::i18n::t(&self.language, "error_self_call");
                        return Command::none(); // Blocage auto-appel
                    }
                }

                if self.active_call.is_some() {
                    let _ = self.handle_call(Message::HangUpCall);
                }

                if target_id.starts_with("grp_") {
                        self.status_message = crate::ui::i18n::t(&self.language, "status_call_group");
                    if let Some(vd) = &self.vault_data {
                        if let Some(group) = vd.groups.get(&target_id) {
                            let my_id = crate::crypto::derive_public_id(&vd.private_key);

                            let tx = self.tx_network.clone();
                            let members = group.members.clone();
                            let target_id_clone = target_id.clone();

                            SOUND_MANAGER.lock().unwrap().start_outgoing_call();
                            self.active_call = Some((target_id.clone(), group.name.clone()));
                            self.active_call_participants.clear();
                              self.call_start_time = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
                            self.chat_history.clear();

                            if let Some(history) = vd.chat_history.get(&target_id) {
                                for msg in history {
                                    self.chat_history.push((msg.author.clone(), msg.content.clone()));
                                }
                            }

                            return Command::perform(
                                async move {
                                    for member_id in members {
                                        if member_id != my_id {
                                            let _ = tx.send(format!("CALL:{}|GRP:{}", member_id, target_id_clone));
                                            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                                        }
                                    }
                                },
                                |_| Message::ResetInactivity
                            );
                        }
                    }
                } else {
                    self.peer_id_input = target_id.clone();
                        self.status_message = crate::ui::i18n::t(&self.language, "status_call_waiting");
                    SOUND_MANAGER.lock().unwrap().start_outgoing_call();
                    let _ = self.tx_network.send(format!("CALL:{}", target_id));
                }
            }
            Message::AcceptCall(id, sdp, grp_id) => {
                let mut pseudo = self.incoming_call.as_ref().unwrap().1.clone();
                let mut actual_id = id.clone();

                SOUND_MANAGER.lock().unwrap().stop_incoming_call();
                SOUND_MANAGER.lock().unwrap().stop_incoming_call();
                SOUND_MANAGER.lock().unwrap().play_red_button();
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                        self.status_message = crate::ui::i18n::t(&self.language, "status_call_secure");
                let _ = self.tx_network.send(format!("ACCEPT:{}:{}", id, sdp));
                
                let mut already_accepted = std::collections::HashSet::new();
                already_accepted.insert(id.clone());
                if !grp_id.is_empty() {
                    for (queued_id, queued_sdp) in self.queued_group_offers.drain(..) {
                        already_accepted.insert(queued_id.clone());
                        let _ = self.tx_network.send(format!("ACCEPT:{}:{}", queued_id, queued_sdp));
                    }
                } else {
                    self.queued_group_offers.clear();
                }

                if let Some(vd) = &self.vault_data {
                    if !grp_id.is_empty() {
                        if let Some(grp) = vd.groups.get(&grp_id) {
                            actual_id = grp_id.clone();
                            pseudo = grp.name.clone();
                            
                            let members_clone = grp.members.clone();
                            let my_pub_id = crate::crypto::derive_public_id(&vd.private_key);
                            let target_grp_clone = grp_id.clone();
                            let tx = self.tx_network.clone();
                            let _caller_id = id.clone();
                            
                            tokio::spawn(async move {
                                for member_id in members_clone {
                                    if member_id != my_pub_id && !already_accepted.contains(&member_id) && my_pub_id > member_id {
                                        let _ = tx.send(format!("CALL:{}|GRP:{}", member_id, target_grp_clone));
                                        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                                    }
                                }
                            });
                        }
                    }
                }

                self.active_call = Some((actual_id.clone(), pseudo));
                self.active_call_participants.clear();
                self.call_start_time = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
                self.chat_input.clear();
                self.chat_history.clear();

                if let Some(vd) = &self.vault_data {
                    if let Some(history) = vd.chat_history.get(&actual_id) {
                        for msg in history {
                            self.chat_history.push((msg.author.clone(), msg.content.clone()));
                        }
                    }
                }
            }
            Message::RejectCall(id) => {
                SOUND_MANAGER.lock().unwrap().stop_incoming_call();
                SOUND_MANAGER.lock().unwrap().stop_incoming_call();
                SOUND_MANAGER.lock().unwrap().play_red_button();
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.status_message = crate::ui::i18n::t(&self.language, "status_call_rejected");
                let _ = self.tx_network.send(format!("REJECT:{}", id));
            }
            Message::HangUpCall => {
                SOUND_MANAGER.lock().unwrap().stop_outgoing_call();
                SOUND_MANAGER.lock().unwrap().play_red_button();
                if let Some((id, _)) = self.active_call.take() {
                    if id.starts_with("grp_") {
                        if let Some(vd) = &self.vault_data {
                            if let Some(group) = vd.groups.get(&id) {
                                let my_id = crate::crypto::derive_public_id(&vd.private_key);
                                // --- CORRECTION DU RACCROCHAGE ---
                                // On ordonne expressément au réseau de couper les ponts individuels
                                // de chaque membre de ce groupe pour purger ton instance de WebRTC !
                                let tx = self.tx_network.clone();
                                let members = group.members.clone();
                                 let grp_id = id.clone();
                                tokio::spawn(async move {
                                    for member_id in &members {
                                        if member_id != &my_id {
                                            let _ = tx.send(format!("CHAT_SEND:{}:SYS:HANGUP", member_id));
                                        }
                                    }
                                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                                    for member_id in &members {
                                        if member_id != &my_id {
                                            let _ = tx.send(format!("HANGUP:{}", member_id));
                                        }
                                    }
                                    // Attend que Kakolookiyam détruise les vrais canaux P2P
                                    tokio::time::sleep(std::time::Duration::from_millis(350)).await;
                                    // Refabrique instantanément les canaux de "Chat" fantômes pour écouter si le groupe parle ou est en appel !
                                    for member_id in &members {
                                        if member_id != &my_id {
                                            let _ = tx.send(format!("CHAT_SEND:{}:SYS:SYNC_WAKEUP:{}", member_id, grp_id));
                                        }
                                    }
                                });
                            }
                        }
                    } else {
                        let tx = self.tx_network.clone();
                        let target_id = id.clone();
                        tokio::spawn(async move {
                            let _ = tx.send(format!("CHAT_SEND:{}:SYS:HANGUP", target_id));
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                            let _ = tx.send(format!("HANGUP:{}", target_id));
                        });
                    }
                    
                    if let Some(start) = self.call_start_time.take() {
                        let now_s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                        let duration = now_s.saturating_sub(start);
                        if let Some(vd) = &mut self.vault_data {
                            let entry = crate::crypto::MessageEntry {
                                author: "SYS:AUTHOR".to_string(),
                                content: format!("SYS:MSG:CALL_ENDED:{}", duration),
                                is_media: false,
                                media_key: None,
                                media_path: None,
                            signature: None,
                                timestamp: now_s,
                            };
                            vd.chat_history.entry(id.clone()).or_default().push(entry);
                            self.needs_save = true;
                        }
                    }
                    self.status_message = crate::ui::i18n::t(&self.language, "status_call_ended");
                    
                    self.is_muted = false;
                    self.chat_input.clear();
                    self.chat_history.clear();
                    let _ = self.tx_network.send("MUTE:off".to_string());
                    
                    let target_id = id.clone();
                    return Command::perform(async move { target_id }, Message::SelectChat);
                }

                self.is_muted = false;
                self.chat_input.clear();
                self.chat_history.clear();
                let _ = self.tx_network.send("MUTE:off".to_string());
            }
            Message::ToggleMute => {
                self.is_muted = !self.is_muted;
                if self.is_muted {
                    SOUND_MANAGER.lock().unwrap().play_mic_muted();
                    let _ = self.tx_network.send("MUTE:on".to_string());
                } else {
                    SOUND_MANAGER.lock().unwrap().play_mic_unmuted();
                    let _ = self.tx_network.send("MUTE:off".to_string());
                }
            }
            _ => {}
        }
        Command::none()
    }
}






