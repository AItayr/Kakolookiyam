use iced::{clipboard, Command};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::crypto;

impl KakolookiyamApp {
    pub(crate) fn handle_chat(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SelectChat(id) => {
                self.idle_seconds = 0;
                self.selected_chat = Some(id.clone());
                self.chat_input.clear();
                self.status_message.clear();

                self.chat_history.clear();
                if let Some(vd) = &self.vault_data {
                    if let Some(history) = vd.chat_history.get(&id) {
                        for msg in history {
                            self.chat_history.push((msg.author.clone(), msg.content.clone()));
                        }
                    }
                }
            }

            Message::DeselectChat => {
                self.idle_seconds = 0;
                self.selected_chat = None;
                self.chat_history.clear();
            }

            Message::ChatInputChanged(val) => {
                self.idle_seconds = 0;
                self.chat_input = val;
            }

            Message::SendChatMessage => {
                self.idle_seconds = 0;
                let text = self.chat_input.trim().to_string();
                if !text.is_empty() {
                    let target = if let Some((active_id, _)) = &self.active_call {
                        Some(active_id.clone())
                    } else {
                        self.selected_chat.clone()
                    };

                    if let Some(target_id) = target {
                        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                            let my_id = crate::crypto::derive_public_id(&vd.private_key);

                            let mut broadcast_cmd = Command::none();

                            if target_id.starts_with("grp_") {
                                if let Some(group) = vd.groups.get(&target_id) {
                                    let tx = self.tx_network.clone();
                                    let members = group.members.clone();
                                    let t_id = target_id.clone();
                                    let msg_text = text.clone();

                                    // --- LE CORRECTIF : Staggering de 50ms pour éviter la perte de paquets ---
                                    broadcast_cmd = Command::perform(
                                        async move {
                                            for member_id in members {
                                                if member_id != my_id {
                                                    let _ = tx.send(format!("CHAT_SEND:{}:SYS:GRP_MSG:{}:{}", member_id, t_id, msg_text));
                                                    // On laisse souffler le réseau 50ms entre chaque envoi !
                                                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                                }
                                            }
                                        },
                                        |_| Message::ResetInactivity
                                    );
                                }
                            } else {
                                let _ = self.tx_network.send(format!("CHAT_SEND:{}:{}", target_id, text));
                            }

                            let entry = crate::crypto::MessageEntry {
                                author: "Moi".to_string(),
                                content: text.clone(),
                                timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                is_media: false,
                                media_key: None,
                                media_path: None,
                            };
                            vd.chat_history.entry(target_id.clone()).or_default().push(entry);
                            let _ = crate::crypto::save_vault(pwd, vd);

                            self.chat_history.push(("Moi".to_string(), text));
                            self.chat_input.clear();

                            return broadcast_cmd; // On retourne la commande asynchrone pour que le délai s'applique.
                        }
                    }
                }
            }

            Message::CopyIdClicked => {
                self.idle_seconds = 0;
                if let Some(vd) = &self.vault_data {
                    return clipboard::write(crypto::derive_public_id(&vd.private_key));
                }
            }

            Message::CopyContactId(id) => {
                self.idle_seconds = 0;
                self.status_message = "✅ ID copié dans le presse-papiers !".to_string();
                return clipboard::write(id);
            }

            _ => {}
        }
        Command::none()
    }
}