use secrecy::ExposeSecret;
use iced::Task as Command;
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
                return self.trigger_global_sync();
            }

            if msg == "ERROR:ALREADY_CONNECTED" {
                return Command::perform(async {}, |_| {
                    Message::ForceDisconnect("❌ Session déjà en cours.".to_string())
                });
            }

            if msg == "ERROR:SERVER_OFFLINE" {
                return Command::perform(async {}, |_| {
                    Message::ForceDisconnect("❌ Serveur injoignable.".to_string())
                });
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
                        if decoded.len() == 32 {
                            key_bytes.copy_from_slice(&decoded);
                        }
                    }

                    if filename.starts_with("SYNC|") {
                        let sync_parts: Vec<&str> = filename.splitn(5, '|').collect();
                        if sync_parts.len() == 5 {
                            let target_id = sync_parts[1].to_string();
                            let ts_str = sync_parts[2];
                            let author = sync_parts[3].to_string();
                            let b64_content = sync_parts[4];

                            if let Ok(timestamp) = ts_str.parse::<u64>() {
                                if let Ok(decoded_bytes) = BASE64_STANDARD.decode(b64_content) {
                                    if let Ok(content_str) = String::from_utf8(decoded_bytes) {
                                        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {

                                            let mut modified = false;
                                            let mut newly_added = false;

                                            {
                                                let history = vd.chat_history.entry(target_id.clone()).or_default();

                                                if let Some(existing) = history.iter_mut().find(|m| m.timestamp == timestamp && m.author == author && m.content == content_str) {
                                                    if existing.media_path.as_deref() != Some(&path) {
                                                        existing.media_path = Some(path.clone());
                                                        existing.media_key = Some(key_bytes);
                                                        modified = true;
                                                    }
                                                } else {
                                                    let entry = crypto::MessageEntry {
                                                        author: author.clone(),
                                                        content: content_str.clone(),
                                                        timestamp,
                                                        is_media: true,
                                                        media_key: Some(key_bytes),
                                                        media_path: Some(path.clone()),
                                                    };
                                                    history.push(entry);
                                                    history.sort_by_key(|m| m.timestamp);
                                                    modified = true;
                                                    newly_added = true;
                                                }
                                            }

                                            if modified {
                                                let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                                                let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_id)
                                                                       || self.active_call.as_ref().map(|(id, _)| id) == Some(&target_id);

                                                if is_currently_viewed {
                                                    self.chat_history.clear();
                                                    if let Some(history) = vd.chat_history.get(&target_id) {
                                                        for msg in history {
                                                            self.chat_history.push((msg.author.clone(), msg.content.clone()));
                                                        }
                                                    }
                                                } else if newly_added {
                                                    *self.unread_counts.entry(target_id.clone()).or_insert(0) += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        return Command::none();
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

                    let sender_pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts.get(&sender_id).cloned().unwrap_or_else(|| "Inconnu".to_string())
                    } else {
                        "Inconnu".to_string()
                    };

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
                        let _ = crypto::save_vault(pwd.expose_secret(), vd);
                    }

                    let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_chat_id)
                                           || self.active_call.as_ref().map(|(id, _)| id) == Some(&target_chat_id);

                    if is_currently_viewed {
                        self.chat_history.push((sender_pseudo, format!("📎 Fichier reçu : {}", display_filename)));
                    } else {
                        *self.unread_counts.entry(target_chat_id.clone()).or_insert(0) += 1;
                    }
                }
            }
                                    else if msg.starts_with("CHAT_RECV:") {
                let parts: Vec<&str> = msg.splitn(3, ':').collect();

                if parts.len() == 3 {
                    let sender_id = parts[1].trim().to_string();
                    let text = parts[2].to_string();

                    // --- NOUVEAU : On étouffe le signal de réveil réseau ---
                    if text.starts_with("SYS:SYNC_WAKEUP") {
                        return Command::none();
                    }

                    if text.starts_with("SYS:SYNC_REQ:") {
                        let sys_parts: Vec<&str> = text.splitn(4, ':').collect();
                        if sys_parts.len() == 4 {
                            let t_id = sys_parts[2].trim().to_string();
                            if let Ok(last_ts) = sys_parts[3].trim().parse::<u64>() {
                                return self.handle_sync_request(sender_id, t_id, last_ts);
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:SYNC_RES:") {
                        let sys_parts: Vec<&str> = text.splitn(6, ':').collect();
                        if sys_parts.len() == 6 {
                            let t_id = sys_parts[2].trim().to_string();
                            let ts_str = sys_parts[3].trim();
                            let msg_type = sys_parts[4].trim();
                            let payload = sys_parts[5];

                            if let Ok(timestamp) = ts_str.parse::<u64>() {
                                use base64::prelude::*;

                                let mut parsed_author = String::new();
                                let mut parsed_content = String::new();
                                let mut parsed_key = None;
                                let mut parsed_path = None;
                                let mut is_media = false;

                                if msg_type == "TXT" {
                                    if let Some((author, b64_content)) = payload.split_once('|') {
                                        parsed_author = author.trim().to_string();
                                        if let Ok(decoded_bytes) = BASE64_STANDARD.decode(b64_content.trim()) {
                                            if let Ok(content_str) = String::from_utf8(decoded_bytes) {
                                                parsed_content = content_str;
                                            }
                                        }
                                    }
                                } else if msg_type == "MED" {
                                    let p: Vec<&str> = payload.split('|').collect();
                                    if p.len() == 4 {
                                        parsed_author = p[0].trim().to_string();
                                        if let Ok(decoded_bytes) = BASE64_STANDARD.decode(p[1].trim()) {
                                            if let Ok(content_str) = String::from_utf8(decoded_bytes) {
                                                parsed_content = content_str;
                                            }
                                        }
                                        let mut key_bytes = [0u8; 32];
                                        if let Ok(decoded_key) = BASE64_STANDARD.decode(p[2].trim()) {
                                            if decoded_key.len() == 32 {
                                                key_bytes.copy_from_slice(&decoded_key);
                                                parsed_key = Some(key_bytes);
                                            }
                                        }
                                        parsed_path = Some(p[3].trim().to_string());
                                        is_media = true;
                                    }
                                }

                                if !parsed_author.is_empty() && !parsed_content.is_empty() {
                                    if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                                        let entry = crypto::MessageEntry {
                                            author: parsed_author.clone(),
                                            content: parsed_content.clone(),
                                            timestamp,
                                            is_media,
                                            media_key: parsed_key,
                                            media_path: parsed_path,
                                        };

                                        let is_new = {
                                            let history = vd.chat_history.entry(t_id.clone()).or_default();
                                            if !history.iter().any(|m| m.timestamp == timestamp && m.author == parsed_author && m.content == parsed_content) {
                                                history.push(entry);
                                                history.sort_by_key(|m| m.timestamp);
                                                true
                                            } else {
                                                false
                                            }
                                        };

                                        if is_new {
                                            let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
                                            let is_currently_viewed = self.selected_chat.as_ref() == Some(&t_id)
                                                                   || self.active_call.as_ref().map(|(id, _)| id) == Some(&t_id);

                                            if is_currently_viewed {
                                                self.chat_history.clear();
                                                if let Some(history) = vd.chat_history.get(&t_id) {
                                                    for msg in history {
                                                        self.chat_history.push((msg.author.clone(), msg.content.clone()));
                                                    }
                                                }
                                            } else {
                                                *self.unread_counts.entry(t_id.clone()).or_insert(0) += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:CALL_CONTEXT:") {
    let sys_parts: Vec<&str> = text.splitn(3, ':').collect();

    if sys_parts.len() == 3 {
        let grp_id = sys_parts[2].trim().to_string();

        let mut already_in_group = false;
        if let Some((active_id, _)) = &self.active_call {
            if active_id == &grp_id {
                already_in_group = true;
            }
        }

        if !already_in_group {
            if let Some(vd) = &self.vault_data {
                if let Some(group) = vd.groups.get(&grp_id) {
                    let my_id = crate::crypto::derive_public_id(&vd.private_key);

                    self.active_call = Some((grp_id.clone(), group.name.clone()));
                    self.chat_history.clear();

                    if let Some(history) = vd.chat_history.get(&grp_id) {
                        for msg in history {
                            self.chat_history.push((msg.author.clone(), msg.content.clone()));
                        }
                    }
                    self.status_message = format!("🚀 Conférence rejointe : {}", group.name);

                    let tx = self.tx_network.clone();
                    let members = group.members.clone();
                    let m_id = my_id.clone();
                    let inviter = sender_id.clone();

                    // Synchronisation initiale pour l'arrivée
                    let sync_task = self.trigger_history_sync(&grp_id);
                    let dial_task = Command::perform(async move {
                        for member_id in members {
                            if member_id != m_id && member_id != inviter {
                                if m_id > member_id {
                                    let _ = tx.send(format!("CALL:{}", member_id));
                                    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                }
                            }
                        }
                    }, |_| Message::ResetInactivity);

                    return Command::batch(vec![dial_task, sync_task]);
                }
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

                                vd.groups.insert(grp_id.clone(), crate::crypto::GroupData {
                                    name: grp_name.clone(),
                                    members
                                });
                                let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);

                                if is_new {
                                    self.status_message = format!("✅ Invité dans le serveur {} !", grp_name);
                                }
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:GROUP_LEAVE:") {
                        let sys_parts: Vec<&str> = text.splitn(3, ':').collect();

                        if sys_parts.len() == 3 {
                            let grp_id = sys_parts[2].trim().to_string();

                            if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                                if let Some(group) = vd.groups.get_mut(&grp_id) {
                                    group.members.retain(|m| m.trim() != sender_id);
                                    let _ = crate::crypto::save_vault(pwd.expose_secret(), vd);
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

                    let sender_pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts.get(&sender_id).cloned().unwrap_or_else(|| "Inconnu".to_string())
                    } else {
                        "Inconnu".to_string()
                    };

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
                        let _ = crypto::save_vault(pwd.expose_secret(), vd);
                    }

                    let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_chat_id)
                                           || self.active_call.as_ref().map(|(id, _)| id) == Some(&target_chat_id);

                    if is_currently_viewed {
                        self.chat_history.push((sender_pseudo, display_text));
                    } else {
                        *self.unread_counts.entry(target_chat_id.clone()).or_insert(0) += 1;
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
                            let _ = crypto::save_vault(pwd.expose_secret(), vd);
                        }
                    }
                }
            }
            else if msg.starts_with("INCOMING_CALL:") {
                let parts: Vec<&str> = msg.splitn(3, ':').collect();

                if parts.len() == 3 {
                    let caller_id = parts[1].trim().to_string();
                    let sdp = parts[2].to_string();

                                        if let Some((active_id, _)) = &self.active_call {
                        if active_id.starts_with("grp_") {
                            if let Some(vd) = &self.vault_data {
                                if let Some(group) = vd.groups.get(active_id) {
                                    if group.members.contains(&caller_id) {
                                        let _ = self.tx_network.send(format!("ACCEPT:{}:{}", caller_id, sdp));
                                        return Command::none();
                                    }
                                }
                            }
                        } else {
                            if let Some(vd) = &self.vault_data {
                                for (_grp_id, group) in &vd.groups {
                                    if group.members.contains(active_id) && group.members.contains(&caller_id) {
                                        let _ = self.tx_network.send(format!("ACCEPT:{}:{}", caller_id, sdp));
                                        return Command::none();
                                    }
                                }
                            }
                        }
                    }

                    let caller_pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts.get(&caller_id).cloned().unwrap_or_else(|| "Inconnu".to_string())
                    } else {
                        "Inconnu".to_string()
                    };

                    self.incoming_call_timer = 0;
                    self.incoming_call = Some((caller_id, caller_pseudo, sdp));
                }
            }
            else if msg.starts_with("CALL_ACTIVE:") {
                let id = msg.trim_start_matches("CALL_ACTIVE:").trim().to_string();

                if let Some((active_id, _)) = &self.active_call {
                                        if active_id.starts_with("grp_") {
                        let _ = self.tx_network.send(format!("CHAT_SEND:{}:SYS:CALL_CONTEXT:{}", id, active_id));
                        return Command::none(); // STOP THE SYNC STORM !
                    }
                } else {
                    let pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts.get(&id).cloned().unwrap_or_else(|| "Ami".to_string())
                    } else {
                        "Ami".to_string()
                    };

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
                    return self.trigger_history_sync(&id);
                }
            }
            else if msg.starts_with("CALL_ENDED:") {
                let id = msg.trim_start_matches("CALL_ENDED:").trim().to_string();
                let _ = self.tx_network.send(format!("HANGUP:{}", id));

                let mut should_end = false;

                if let Some((active_id, _)) = &self.active_call {
                    if active_id == &id {
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
            else if msg.starts_with("TIMEOUT:") {
                let tgt = msg.trim_start_matches("TIMEOUT:");
                if self.active_call.is_none() {
                    self.status_message = "L'interlocuteur n'est pas disponible.".to_string();
                    let _ = self.tx_network.send(format!("HANGUP:{}", tgt));
                }
            }
            else if msg.starts_with("LOADING:") {
                // Ignore late loading spams if call is already connected or failed
                if self.active_call.is_none() {
                    self.status_message = msg;
                }
            }
            else {
                self.status_message = msg;
            }
        }
        Command::none()
    }
}