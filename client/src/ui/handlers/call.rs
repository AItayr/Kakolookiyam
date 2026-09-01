use iced::Task as Command;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn handle_call(&mut self, message: Message) -> Command<Message> {
        self.idle_seconds = 0;

        match message {
            Message::PeerIdChanged(val) => {
                self.peer_id_input = val;
            }
            Message::ConnectClicked => {
                self.status_message = "🔗 En attente de l'interlocuteur...".to_string();
                let _ = self.tx_network.send(format!("CALL:{}", self.peer_id_input));
            }
            Message::CallContact(target_id) => {
                if target_id.starts_with("grp_") {
                    self.status_message = "📞 Conférence de groupe en cours...".to_string();
                    if let Some(vd) = &self.vault_data {
                        if let Some(group) = vd.groups.get(&target_id) {
                            let my_id = crate::crypto::derive_public_id(&vd.private_key);

                            let tx = self.tx_network.clone();
                            let members = group.members.clone();

                            self.active_call = Some((target_id.clone(), group.name.clone()));
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
                                            let _ = tx.send(format!("CALL:{}", member_id));
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
                    self.status_message = "🔗 En attente de l'interlocuteur...".to_string();
                    let _ = self.tx_network.send(format!("CALL:{}", target_id));
                }
            }
            Message::AcceptCall(id, sdp) => {
                let pseudo = self.incoming_call.as_ref().unwrap().1.clone();

                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.status_message = "🔗 Connexion sécurisée en cours...".to_string();
                let _ = self.tx_network.send(format!("ACCEPT:{}:{}", id, sdp));

                self.active_call = Some((id.clone(), pseudo));
                self.chat_input.clear();
                self.chat_history.clear();

                if let Some(vd) = &self.vault_data {
                    if let Some(history) = vd.chat_history.get(&id) {
                        for msg in history {
                            self.chat_history.push((msg.author.clone(), msg.content.clone()));
                        }
                    }
                }
            }
            Message::RejectCall(id) => {
                self.incoming_call = None;
                self.incoming_call_timer = 0;
                self.status_message = "❌ Appel rejeté.".to_string();
                let _ = self.tx_network.send(format!("REJECT:{}", id));
            }
            Message::HangUpCall => {
                if let Some((id, _)) = self.active_call.take() {
                    if id.starts_with("grp_") {
                        if let Some(vd) = &self.vault_data {
                            if let Some(group) = vd.groups.get(&id) {
                                let my_id = crate::crypto::derive_public_id(&vd.private_key);
                                // --- CORRECTION DU RACCROCHAGE ---
                                // On ordonne expressément au réseau de couper les ponts individuels
                                // de chaque membre de ce groupe pour purger ton instance de WebRTC !
                                for member_id in &group.members {
                                    if member_id != &my_id {
                                        let _ = self.tx_network.send(format!("HANGUP:{}", member_id));
                                    }
                                }
                            }
                        }
                    } else {
                        let _ = self.tx_network.send(format!("HANGUP:{}", id));
                    }
                    self.status_message = "Appel terminé.".to_string();
                }

                self.is_muted = false;
                self.chat_input.clear();
                self.chat_history.clear();
                let _ = self.tx_network.send("MUTE:off".to_string());
            }
            Message::ToggleMute => {
                self.is_muted = !self.is_muted;
                if self.is_muted {
                    let _ = self.tx_network.send("MUTE:on".to_string());
                } else {
                    let _ = self.tx_network.send("MUTE:off".to_string());
                }
            }
            _ => {}
        }
        Command::none()
    }
}