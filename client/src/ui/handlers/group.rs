use crate::sound::SOUND_MANAGER;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use iced::Task as Command;
use secrecy::ExposeSecret;

impl KakolookiyamApp {
    pub(crate) fn handle_group(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::NewGroupInputChanged(val) => {
                self.new_group_input = val;
            }

            Message::CreateGroup => {
                let name = self.new_group_input.trim().to_string();
                if !name.is_empty() {
                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                        use rand::RngCore;
                        let mut random_bytes = [0u8; 8];
                        rand::rngs::OsRng.fill_bytes(&mut random_bytes);

                        let mut hex = String::new();
                        for b in random_bytes {
                            std::fmt::Write::write_fmt(&mut hex, format_args!("{:02x}", b))
                                .unwrap();
                        }

                        let group_id = format!("grp_{}", hex);
                        let my_id = crate::crypto::derive_public_id(&vd.private_key);

                        vd.groups.insert(
                            group_id.clone(),
                            crate::crypto::GroupData {
                                name,
                                members: vec![my_id],
                            },
                        );

                        let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);

                        self.new_group_input.clear();
                        self.selected_chat = Some(group_id);
                        self.chat_history.clear();
                        self.show_group_options = false;
                    }
                }
            }

            Message::NewMemberInputChanged(val) => {
                self.new_member_input = val;
            }

            Message::AddMemberToGroup => {
                let new_member = self.new_member_input.trim().to_string();
                if !new_member.is_empty() {
                    if let Some(grp_id) = &self.selected_chat {
                        if grp_id.starts_with("grp_") {
                            if let (Some(vd), Some(pwd)) =
                                (&mut self.vault_data, &self.master_password)
                            {
                                let mut sync_data = None;

                                if let Some(group) = vd.groups.get_mut(grp_id) {
                                    if !group.members.contains(&new_member) {
                                        group.members.push(new_member.clone());
                                        sync_data =
                                            Some((group.name.clone(), group.members.clone()));
                                    } else {
                                        self.status_message = crate::ui::i18n::t(
                                            &self.language,
                                            "status_member_exists",
                                        );
                                    }
                                }

                                if let Some((grp_name, members)) = sync_data {
                                    let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                                    let members_str = members.join(",");
                                    let my_id = crate::crypto::derive_public_id(&vd.private_key);

                                    for member_id in &members {
                                        if member_id != &my_id {
                                            let _ = self.tx_network.send(format!(
                                                "CHAT_SEND:{}:SYS:GROUP_SYNC:{}:{}:{}",
                                                member_id, grp_id, grp_name, members_str
                                            ));
                                        }
                                    }
                                    self.status_message =
                                        crate::ui::i18n::t(&self.language, "status_member_invited");
                                }
                                self.new_member_input.clear();
                            }
                        }
                    }
                }
            }

            Message::AddSpecificMemberToGroup(new_member_id) => {
                self.idle_seconds = 0;
                let clean_id = new_member_id.trim().to_string();

                if let Some(grp_id) = &self.selected_chat {
                    if grp_id.starts_with("grp_") {
                        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password)
                        {
                            let mut sync_data = None;

                            if let Some(group) = vd.groups.get_mut(grp_id) {
                                if !group.members.contains(&clean_id) {
                                    group.members.push(clean_id.clone());
                                    sync_data = Some((group.name.clone(), group.members.clone()));
                                } else {
                                    self.status_message =
                                        crate::ui::i18n::t(&self.language, "status_member_exists");
                                }
                            }

                            if let Some((grp_name, members)) = sync_data {
                                let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                                let members_str = members.join(",");
                                let my_id = crate::crypto::derive_public_id(&vd.private_key);

                                for member_id in &members {
                                    if member_id != &my_id {
                                        let _ = self.tx_network.send(format!(
                                            "CHAT_SEND:{}:SYS:GROUP_SYNC:{}:{}:{}",
                                            member_id, grp_id, grp_name, members_str
                                        ));
                                    }
                                }
                                self.status_message =
                                    crate::ui::i18n::t(&self.language, "status_contact_synced");
                            }
                        }
                    }
                }
            }

            Message::DeleteGroup => {
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .play_red_button();
                self.idle_seconds = 0;
                let target_id = self.selected_chat.clone();

                if let Some(grp_id) = target_id {
                    if grp_id.starts_with("grp_") {
                        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password)
                        {
                            if let Some(group) = vd.groups.get(&grp_id) {
                                let my_id = crate::crypto::derive_public_id(&vd.private_key);
                                let is_creator = if let Some(group) = vd.groups.get(&grp_id) {
                                    group.members.first() == Some(&my_id)
                                } else {
                                    false
                                };
                                for member_id in &group.members {
                                    if member_id != &my_id {
                                        if is_creator {
                                            let _ = self.tx_network.send(format!(
                                                "CHAT_SEND:{}:SYS:GRP_DEL:{}",
                                                member_id, grp_id
                                            ));
                                        } else {
                                            let _ = self.tx_network.send(format!(
                                                "CHAT_SEND:{}:SYS:GRP_MSG:{}:SYS:EVT:LEAVE:{}:{}",
                                                member_id,
                                                grp_id,
                                                my_id,
                                                std::time::SystemTime::now()
                                                    .duration_since(std::time::UNIX_EPOCH)
                                                    .unwrap()
                                                    .as_secs()
                                            ));
                                        }
                                    }
                                }
                            }

                            vd.groups.remove(&grp_id);
                            vd.chat_history.remove(&grp_id);
                            vd.tombstones.insert(grp_id.clone());
                            let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);

                            self.selected_chat = None;
                            self.chat_history.clear();
                            self.show_group_options = false;
                            self.status_message =
                                crate::ui::i18n::t(&self.language, "status_group_left");
                        }
                    }
                }
            }
            _ => {}
        }
        Command::none()
    }
}
