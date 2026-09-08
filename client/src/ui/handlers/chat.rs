use secrecy::ExposeSecret;
use iced::{clipboard, Task as Command};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::crypto;

impl KakolookiyamApp {
    pub(crate) fn handle_chat(&mut self, message: Message) -> Command<Message> {
        self.idle_seconds = 0;

        match message {
            Message::SelectChat(id) => {
                self.selected_chat = Some(id.clone());

                // --- PURGE DE LA BULLE DE NOTIFICATION ---
                self.unread_counts.remove(&id);

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

                return self.trigger_history_sync(&id);
            }
            Message::DeselectChat => {
                self.selected_chat = None;
                self.chat_history.clear();
            }
            Message::ChatInputChanged(val) => {
                self.chat_input = val;
            }
            Message::SendChatMessage => {
                let text = self.chat_input.trim().to_string();
                if text.is_empty() { return Command::none(); }

                let target = self.active_call.as_ref().map(|(id, _)| id.clone())
                    .or_else(|| self.selected_chat.clone());

                if let Some(target_id) = target {
                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                        let my_id = crypto::derive_public_id(&vd.private_key);
                        let mut broadcast_cmd = Command::none();

                        if target_id.starts_with("grp_") {
                            if let Some(group) = vd.groups.get(&target_id) {
                                let tx = self.tx_network.clone();
                                let members = group.members.clone();
                                let t_id = target_id.clone();
                                let msg_text = text.clone();

                                broadcast_cmd = Command::perform(
                                    async move {
                                        for member_id in members {
                                            if member_id != my_id {
                                                let _ = tx.send(format!("CHAT_SEND:{}:SYS:GRP_MSG:{}:{}", member_id, t_id, msg_text));
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

                        let entry = crypto::MessageEntry {
                            author: "Moi".to_string(),
                            content: text.clone(),
                            timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                            is_media: false,
                            media_key: None,
                            media_path: None,
                        };

                        vd.chat_history.entry(target_id).or_default().push(entry);
                        let _ = crypto::save_vault(pwd.expose_secret(), vd);

                        self.chat_history.push(("Moi".to_string(), text));
                        self.chat_input.clear();

                        return broadcast_cmd;
                    }
                }
            }
            Message::CopyIdClicked => {
                if let Some(vd) = &self.vault_data {
                    return clipboard::write(crypto::derive_public_id(&vd.private_key));
                }
            }
            Message::CopyContactId(id) => {
                self.status_message = "✅ ID copié dans le presse-papiers !".to_string();
                return clipboard::write(id);
            }
            _ => {}
        }
        Command::none()
    }
}