use crate::crypto;
use crate::sound::SOUND_MANAGER;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use iced::{Task as Command, clipboard};
use secrecy::ExposeSecret;

impl KakolookiyamApp {
    pub(crate) fn handle_chat(&mut self, message: Message) -> iced::Task<Message> {
        self.idle_seconds = 0;

        match message {
            Message::SelectChat(id) => {
                self.selected_chat = Some(id.clone());
                self.show_volume_panel = false;
                self.show_group_options = false;

                // --- PURGE DE LA BULLE DE NOTIFICATION ---
                self.unread_counts.remove(&id);

                self.chat_input.clear();
                self.status_message.clear();
                self.chat_history.clear();

                if let Some(vd) = &self.vault_data {
                    if let Some(history) = vd.chat_history.get(&id) {
                        for msg in history {
                            self.chat_history
                                .push((msg.author.clone(), msg.content.clone()));
                        }
                    }

                    if id.starts_with("grp_") {
                        if let Some(group) = vd.groups.get(&id) {
                            let tx = self.tx_network.clone();
                            let my_id = crate::crypto::derive_public_id(&vd.private_key);
                            for member_id in &group.members {
                                if member_id != &my_id {
                                    let _ = tx.send(format!(
                                        "CHAT_SEND:{}:SYS:SYNC_WAKEUP:{}",
                                        member_id, id
                                    ));
                                }
                            }
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
                let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let text = self.chat_input.trim().to_string();
                if text.is_empty() {
                    return iced::Task::none();
                }

                let target = self
                    .active_call
                    .as_ref()
                    .map(|(id, _)| id.clone())
                    .or_else(|| self.selected_chat.clone());

                if let Some(target_id) = target {
                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                        let my_id = crypto::derive_public_id(&vd.private_key);
                        let mut broadcast_cmd = iced::Task::none();

                        if target_id.starts_with("grp_") {
                            if let Some(group) = vd.groups.get(&target_id) {
                                let tx = self.tx_network.clone();
                                let members = group.members.clone();
                                let t_id = target_id.clone();
                                let msg_text = text.clone();

                                let sig = crypto::sign_message(&vd.private_key, &t_id, timestamp, &msg_text);
                                broadcast_cmd = Command::perform(
                                    async move {
                                        let sends = members
                                            .into_iter()
                                            .filter(|m| m != &my_id)
                                            .map(|member_id| {
                                                let tx = tx.clone();
                                                let msg = format!(
                                                    "CHAT_SEND:{}:SYS:GRP_MSG:{}:{}:{}",
                                                    member_id, t_id, sig, msg_text
                                                );
                                                async move {
                                                    let _ = tx.send(msg);
                                                }
                                            });
                                        futures_util::future::join_all(sends).await;
                                    },
                                    |_| Message::ResetInactivity,
                                );
                            }
                        } else {
                            let _ = self
                                .tx_network
                                .send(format!("CHAT_SEND:{}:{}", target_id, text));
                        }

                        let entry = crypto::MessageEntry {
                            author: crate::ui::i18n::t(&self.language, "me_author"),
                            content: text.clone(),
                            timestamp,
                            is_media: false,
                            media_key: None,
                            media_path: None,
                            signature: Some(crypto::sign_message(
                                &vd.private_key,
                                &target_id,
                                timestamp,
                                &text,
                            )),
                        };

                        vd.chat_history.entry(target_id).or_default().push(entry);
                        let _ = crypto::save_vault(pwd.expose_secret(), vd);

                        self.chat_history
                            .push((crate::ui::i18n::t(&self.language, "me_author"), text));
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
            Message::AcceptRequest(id) => {
                if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                    if let Some(pseudo) = vd.pending_requests.remove(&id) {
                        vd.contacts.insert(id.clone(), pseudo);
                        let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                        self.status_message =
                            crate::ui::i18n::t(&self.language, "status_contact_added");
                    }
                }
            }
            Message::RejectRequest(id) => {
                if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                    SOUND_MANAGER
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .play_red_button();
                    vd.pending_requests.remove(&id);
                    let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                    self.status_message =
                        crate::ui::i18n::t(&self.language, "status_request_rejected");
                }
            }
            Message::BlockContact(id) => return self.handle_block_contact(id),
            Message::UnblockContact(id) => return self.handle_unblock_contact(id),
            Message::VolumeChanged(id, vol) => {
                crate::audio::set_user_volume(id, vol);
                return iced::Task::none();
            }
            Message::CopyContactId(id) => {
                self.status_message = crate::ui::i18n::t(&self.language, "status_id_copied");
                return clipboard::write(id);
            }
            _ => {}
        }
        iced::Task::none()
    }
    pub(crate) fn handle_block_contact(&mut self, id: String) -> iced::Task<Message> {
        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
            SOUND_MANAGER
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .play_red_button();
            vd.pending_requests.remove(&id);
            vd.contacts.remove(&id);
            if self.selected_chat.as_ref() == Some(&id) {
                self.selected_chat = None;
                self.chat_history.clear();
            }
            vd.blocked_ids.insert(id.clone());
            let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);

            // Sync with Router
            let mut sync_str = String::from("BLOCKED_SYNC");
            for b_id in vd.blocked_ids.iter() {
                sync_str.push_str(&format!(":{}", b_id));
            }
            let _ = self.tx_network.send(sync_str);
        }
        iced::Task::none()
    }

    pub(crate) fn handle_unblock_contact(&mut self, id: String) -> iced::Task<Message> {
        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
            if vd.blocked_ids.remove(&id) {
                let sys_author = crate::ui::i18n::t(&self.language, "system_author");
                let mut pseudo = id.clone();
                if let Some(history) = vd.chat_history.get(&id) {
                    if let Some(m) = history.iter().find(|m| {
                        m.author != "SYS:AUTHOR"
                            && m.author != sys_author
                            && m.author != "Moi"
                            && m.author != vd.pseudo
                            && !m.author.is_empty()
                    }) {
                        pseudo = m.author.clone();
                    }
                }
                vd.contacts.insert(id.clone(), pseudo);

                let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);

                // Sync with Router
                let mut sync_str = String::from("BLOCKED_SYNC");
                for b_id in vd.blocked_ids.iter() {
                    sync_str.push_str(&format!(":{}", b_id));
                }
                let _ = self.tx_network.send(sync_str);
            }
        }
        iced::Task::none()
    }
}
