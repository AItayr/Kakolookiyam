use crate::crypto;
use crate::sound::SOUND_MANAGER;
use crate::ui::app::{AppState, KakolookiyamApp};
use crate::ui::messages::Message;
use iced::Task as Command;

impl KakolookiyamApp {
    pub(crate) fn handle_network(&mut self, message: Message) -> Command<Message> {
        if let Message::NetworkEvent(msg) = message {
            self.idle_seconds = 0;

            if msg == "SUCCESS:REGISTERED" {
                self.server_offline_since = None;
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .stop_main_theme();
                self.state = AppState::Unlocked;
                self.clear_auth_fields();
                return self.trigger_global_sync();
            }

            if msg.starts_with("ERROR:BAD_SIGNATURE") {
                SOUND_MANAGER.lock().unwrap_or_else(|e| e.into_inner()).play_error();
                return Command::perform(async {}, |_| {
                    Message::ForceDisconnect("L\'heure de votre ordinateur est incorrecte.".to_string())
                });
            }

                        if msg.starts_with("BOUNCE:") {
                let cmd = msg.trim_start_matches("BOUNCE:");
                let _ = self.tx_network.send(cmd.to_string());
                return Command::none();
            }

            if msg.starts_with("SYS_MSG:") {
                let text = msg.trim_start_matches("SYS_MSG:").to_string();
                self.status_message = text;
                return Command::none();
            }

            if msg == "ERROR:ALREADY_CONNECTED" {
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .play_error();
                return Command::perform(async {}, |_| {
                    Message::ForceDisconnect("❌ Session déjà en cours.".to_string())
                });
            }

                        if msg == "ERROR:SERVER_OFFLINE" {
                if self.vault_data.is_some() {
                    self.server_offline_since.get_or_insert_with(std::time::Instant::now);
                    self.status_message = crate::ui::i18n::t(&self.language, "status_reconnecting");
                    if self.server_offline_since.map_or(false, |t| t.elapsed().as_secs() > 300) {
                        self.status_message = crate::ui::i18n::t(&self.language, "status_server_offline");
                        return Command::none();
                    }
                    return Command::none();
                } else {
                    SOUND_MANAGER.lock().unwrap_or_else(|e| e.into_inner()).play_error();
                    let msg = crate::ui::i18n::t(&self.language, "status_server_offline");
                    return Command::perform(async {}, move |_| {
                        Message::ForceDisconnect(msg.clone())
                    });
                }
            }

            if msg.starts_with("FILE_RECV:") {
                let payload = msg.trim_start_matches("FILE_RECV:");
                let parts: Vec<&str> = payload.splitn(4, '|').collect();
                if parts.len() == 4 {
                    let sender_id = parts[0].trim().to_string();
                    let key_b64 = parts[1].trim().to_string();
                    let path = parts[2].trim().to_string();
                    let filename = parts[3].trim().to_string();

                    use base64::prelude::*;
                    let mut key_bytes = [0u8; 32];
                    if let Ok(decoded) = BASE64_STANDARD.decode(&key_b64) {
                        if decoded.len() == 32 {
                            key_bytes.copy_from_slice(&decoded);
                        }
                    }

                    if filename.starts_with("SYNC|") {
                        let sync_parts: Vec<&str> = filename.splitn(6, '|').collect();
                        if sync_parts.len() >= 5 {
                            let target_id = sync_parts[1].to_string();
                            let ts_str = sync_parts[2];
                            let author = sync_parts[3].to_string();
                            let b64_content = sync_parts[4];
                            let parsed_sig = if sync_parts.len() == 6 {
                                sync_parts[5].to_string()
                            } else {
                                String::new()
                            };

                            if let Ok(timestamp) = ts_str.parse::<u64>() {
                                if let Ok(decoded_bytes) = BASE64_STANDARD.decode(b64_content) {
                                    if let Ok(content_str) = String::from_utf8(decoded_bytes) {
                                        let hash_valid = match content_str.rsplit_once('|') {
                                            Some((_, h)) if h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()) => {
                                                std::fs::read(&path).map(|bytes| {
                                                    let d = ring::digest::digest(&ring::digest::SHA256, &bytes);
                                                    d.as_ref().iter().map(|b| format!("{:02x}", b)).collect::<String>() == h
                                                }).unwrap_or(false)
                                            },
                                            _ => false,
                                        };
                                        if !hash_valid {
                                            let _ = std::fs::remove_file(&path);
                                            return Command::none();
                                        }
                                        if let (Some(vd), Some(_pwd)) =
                                            (&mut self.vault_data, &self.master_password)
                                        {
                                            let author_id =
                                                vd.contacts.iter().find_map(|(id, pseudo)| {
                                                    if pseudo == &author {
                                                        Some(id.clone())
                                                    } else {
                                                        None
                                                    }
                                                });

                                            let is_valid = match author_id {
                                                Some(id) => {
                                                    !parsed_sig.is_empty()
                                                        && crate::crypto::verify_message(&id, &crate::crypto::conv_id(&crate::crypto::derive_public_id(&vd.private_key), &target_id), timestamp, &content_str, &parsed_sig)
                                                }
                                                None => false,
                                            };
                                            if !is_valid {
                                                let _ = std::fs::remove_file(&path);
                                                return Command::none();
                                            }
                                            let mut modified = false;
                                            let mut newly_added = false;

                                            {
                                                let history = vd
                                                    .chat_history
                                                    .entry(target_id.clone())
                                                    .or_default();

                                                if let Some(existing) =
                                                    history.iter_mut().find(|m| {
                                                        m.is_media
                                                            && (m.timestamp.max(timestamp)
                                                                - m.timestamp.min(timestamp)
                                                                <= 5)
                                                    })
                                                {
                                                    if existing.media_path.as_deref() != Some(&path)
                                                    {
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
                                                        signature: if parsed_sig.is_empty() {
                                                            None
                                                        } else {
                                                            Some(parsed_sig.clone())
                                                        },
                                                    };
                                                    history.push(entry);
                                                    history.sort_by_key(|m| m.timestamp);
                                                    modified = true;
                                                    newly_added = true;
                                                }
                                            }

                                            if modified {
                                                self.needs_save = true;
                                                let is_currently_viewed =
                                                    self.selected_chat.as_ref() == Some(&target_id);

                                                if is_currently_viewed {
                                                    self.chat_history.clear();
                                                    if let Some(history) =
                                                        vd.chat_history.get(&target_id)
                                                    {
                                                        for msg in history {
                                                            self.chat_history.push((
                                                                msg.author.clone(),
                                                                msg.content.clone(),
                                                            ));
                                                        }
                                                    }
                                                } else if newly_added {
                                                    *self
                                                        .unread_counts
                                                        .entry(target_id.clone())
                                                        .or_insert(0) += 1;
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
                        vd.contacts
                            .get(&sender_id)
                            .cloned()
                            .unwrap_or_else(|| "Inconnu".to_string())
                    } else {
                        "Inconnu".to_string()
                    };

                    if let (Some(vd), Some(_pwd)) = (&mut self.vault_data, &self.master_password) {
                        let entry = crypto::MessageEntry {
                            author: sender_pseudo.clone(),
                            content: format!(
                                "{} {}",
                                crate::ui::i18n::t(&self.language, "msg_file_received"),
                                display_filename
                            ),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            is_media: true,
                            media_key: Some(key_bytes),
                            media_path: Some(path.clone()),
                            signature: None,
                        };

                        vd.chat_history
                            .entry(target_chat_id.clone())
                            .or_default()
                            .push(entry);
                        self.needs_save = true;
                    }

                    let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_chat_id);

                    SOUND_MANAGER
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .play_message_received();
                    if is_currently_viewed {
                        self.chat_history.push((
                            sender_pseudo,
                            format!(
                                "{} {}",
                                crate::ui::i18n::t(&self.language, "msg_file_received"),
                                display_filename
                            ),
                        ));
                    } else {
                        *self
                            .unread_counts
                            .entry(target_chat_id.clone())
                            .or_insert(0) += 1;
                    }
                }
            } else if msg.starts_with("CHAT_RECV:") {
                let parts: Vec<&str> = msg.splitn(3, ':').collect();

                if parts.len() == 3 {
                    let sender_id = parts[1].trim().to_string();

                    if let Some(vd) = &self.vault_data {
                        if vd.blocked_ids.contains(&sender_id) {
                            return Command::none();
                        }
                    }
                    let text = parts[2].to_string();

                    let is_contact = self.vault_data.as_ref()
                        .map_or(false, |vd| vd.contacts.contains_key(&sender_id));
                    let in_my_group = self.vault_data.as_ref()
                        .map_or(false, |vd| vd.groups.values().any(|g| g.members.contains(&sender_id)));
                    if text.starts_with("SYS:") && !is_contact && !in_my_group {
                        return Command::none();
                    }

                    // --- NOUVEAU : On étouffe le signal de réveil réseau ---
                    if text.starts_with("SYS:SYNC_WAKEUP") {
                        // Si le récepteur est lui-même dans un appel, il renvoie un Heartbeat immédiatement
                        if let Some((active_id, _)) = &self.active_call {
                            if active_id.starts_with("grp_") {
                                let _ = self.tx_network.send(format!(
                                    "CHAT_SEND_IF_OPEN:{}:SYS:GRP_HEARTBEAT:{}",
                                    sender_id, active_id
                                ));
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:GRP_HEARTBEAT:") {
                        let grp_id = text
                            .trim_start_matches("SYS:GRP_HEARTBEAT:")
                            .trim()
                            .to_string();
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        self.group_call_presences
                            .entry(grp_id)
                            .or_default()
                            .insert(sender_id.clone(), now);
                        return Command::none(); // Important silently
                    }

                    if text == "SYS:HANGUP" {
                        self.active_call_participants.remove(&sender_id);
                        let _ = self.tx_network.send(format!("HANGUP:{}", sender_id));
                        let mut should_end = false;
                        if let Some((active_id, _)) = &self.active_call {
                            if active_id == &sender_id {
                                should_end = true;
                            }
                        }
                        if should_end {
                            self.active_call = None;
                            self.active_call_participants.clear();
                            self.is_muted = false;
                            let _ = self.tx_network.send("MUTE:off".to_string());
                            self.chat_input.clear();
                            self.chat_history.clear();
                            self.status_message =
                                crate::ui::i18n::t(&self.language, "status_remote_hangup");
                        }
                        return Command::none();
                    }

                    if text == "SYS:CALL_BUSY" {
                        self.status_message =
                            crate::ui::i18n::t(&self.language, "status_remote_busy");
                        let _ = self.tx_network.send(format!("HANGUP:{}", sender_id));
                        self.active_call = None;
                        return Command::none();
                    }

                    if text.starts_with("SYS:GRP_DEL:") {
                        let sys_parts: Vec<&str> = text.splitn(3, ':').collect();
                        if sys_parts.len() == 3 {
                            let grp_id = sys_parts[2].trim().to_string();
                            if let (Some(vd), Some(_pwd)) =
                                (&mut self.vault_data, &self.master_password)
                            {
                                let mut is_creator = false;
                                if let Some(group) = vd.groups.get(&grp_id) {
                                    if group.members.first() == Some(&sender_id) {
                                        is_creator = true;
                                    }
                                }
                                if is_creator {
                                    vd.groups.remove(&grp_id);
                                    self.needs_save = true;
                                    self.status_message = crate::ui::i18n::t(
                                        &self.language,
                                        "status_server_dissolved",
                                    );
                                    if self.selected_chat.as_ref() == Some(&grp_id) {
                                        self.selected_chat = None;
                                    }
                                }
                            }
                        }
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

                            // [MITIGATION] HIGH-1d: Vrification d'autorisation SYNC_RES
                            let mut authorized = false;
                            if let Some(vd) = &self.vault_data {
                                if t_id.starts_with("grp_") {
                                    if let Some(group) = vd.groups.get(&t_id) {
                                        if group.members.contains(&sender_id) {
                                            authorized = true;
                                        }
                                    }
                                } else if t_id == sender_id {
                                    authorized = true;
                                }
                            }
                            if !authorized {
                                return Command::none();
                            }

                            if let Some(vd) = &self.vault_data {
                                if vd.tombstones.contains(&t_id) {
                                    return Command::none();
                                }
                            }
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

                                let mut parsed_sig = String::new();
                                if msg_type == "TXT" {
                                    let parts: Vec<&str> = payload.split('|').collect();
                                    if parts.len() >= 2 {
                                        parsed_author = parts[0].trim().to_string();
                                        if let Ok(decoded_bytes) =
                                            BASE64_STANDARD.decode(parts[1].trim())
                                        {
                                            if let Ok(content_str) =
                                                String::from_utf8(decoded_bytes)
                                            {
                                                parsed_content = content_str;
                                            }
                                        }
                                    }
                                    if parts.len() >= 3 {
                                        parsed_sig = parts[2].trim().to_string();
                                    }
                                } else if msg_type == "MED" {
                                    let p: Vec<&str> = payload.split('|').collect();
                                    if p.len() >= 4 {
                                        parsed_author = p[0].trim().to_string();
                                        if let Ok(decoded_bytes) =
                                            BASE64_STANDARD.decode(p[1].trim())
                                        {
                                            if let Ok(content_str) =
                                                String::from_utf8(decoded_bytes)
                                            {
                                                parsed_content = content_str;
                                            }
                                        }
                                        let mut key_bytes = [0u8; 32];
                                        if let Ok(decoded_key) = BASE64_STANDARD.decode(p[2].trim())
                                        {
                                            if decoded_key.len() == 32 {
                                                key_bytes.copy_from_slice(&decoded_key);
                                                parsed_key = Some(key_bytes);
                                            }
                                        }
                                        parsed_path = Some(p[3].trim().to_string());
                                        is_media = true;
                                    }
                                    if p.len() >= 5 {
                                        parsed_sig = p[4].trim().to_string();
                                    }
                                }

                                if !parsed_author.is_empty() && !parsed_content.is_empty() {
                                    if let (Some(vd), Some(_pwd)) =
                                        (&mut self.vault_data, &self.master_password)
                                    {
                                        let author_id =
                                            vd.contacts.iter().find_map(|(id, pseudo)| {
                                                if pseudo == &parsed_author {
                                                    Some(id.clone())
                                                } else {
                                                    None
                                                }
                                            });

                                        let is_valid = match author_id {
                                            Some(id) => {
                                                !parsed_sig.is_empty()
                                                    && crate::crypto::verify_message(&id, &crate::crypto::conv_id(&crate::crypto::derive_public_id(&vd.private_key), &t_id), timestamp, &parsed_content, &parsed_sig)
                                            }
                                            None => false,
                                        };

                                        if !is_valid {
                                            return Command::none(); // Falsifi !
                                        }

                                        let entry = crypto::MessageEntry {
                                            author: parsed_author.clone(),
                                            content: parsed_content.clone(),
                                            timestamp,
                                            is_media,
                                            media_key: parsed_key,
                                            media_path: parsed_path,
                                            signature: if parsed_sig.is_empty() {
                                                None
                                            } else {
                                                Some(parsed_sig)
                                            },
                                        };

                                        let is_new = {
                                            let history =
                                                vd.chat_history.entry(t_id.clone()).or_default();
                                            let is_dup = history.iter().any(|m| {
                                                let t_diff = m.timestamp.max(timestamp)
                                                    - m.timestamp.min(timestamp);
                                                if t_diff > 5 {
                                                    return false;
                                                }
                                                if m.content == parsed_content {
                                                    return true;
                                                }
                                                if m.is_media && is_media {
                                                    return true;
                                                }
                                                if (m.author.starts_with(&crate::ui::i18n::t(
                                                    &self.language,
                                                    "system_author",
                                                )) || m.author
                                                    == crate::ui::i18n::t(
                                                        &self.language,
                                                        "me_author",
                                                    ))
                                                    && (parsed_author.starts_with(
                                                        &crate::ui::i18n::t(
                                                            &self.language,
                                                            "system_author",
                                                        ),
                                                    ) || parsed_author
                                                        == crate::ui::i18n::t(
                                                            &self.language,
                                                            "me_author",
                                                        ))
                                                    && m.content.contains("APPEL")
                                                {
                                                    return true;
                                                }
                                                false
                                            });
                                            if !is_dup {
                                                history.push(entry);
                                                history.sort_by_key(|m| m.timestamp);
                                                true
                                            } else {
                                                false
                                            }
                                        };

                                        if is_new {
                                            // -- EVENT SOURCING: Auto-cicatrisation des departs! --
                                            if parsed_content.starts_with("SYS:EVT:LEAVE:") {
                                                let payload = parsed_content
                                                    .trim_start_matches("SYS:EVT:LEAVE:")
                                                    .trim();
                                                let parts: Vec<&str> =
                                                    payload.splitn(2, ':').collect();
                                                if parts.len() == 2 {
                                                    let left_id = parts[0];
                                                    let _ts_str = parts[1];
                                                    if left_id.to_lowercase()
                                                        == parsed_author.to_lowercase()
                                                    {
                                                        if let Some(group) =
                                                            vd.groups.get_mut(&t_id)
                                                        {
                                                            group.members.retain(|m| {
                                                                m.trim().to_lowercase()
                                                                    != left_id.to_lowercase()
                                                            });
                                                        }
                                                    }
                                                }
                                            }

                                            self.needs_save = true;
                                            SOUND_MANAGER
                                                .lock()
                                                .unwrap_or_else(|e| e.into_inner())
                                                .play_message_received();
                                            let is_currently_viewed =
                                                self.selected_chat.as_ref() == Some(&t_id);

                                            if is_currently_viewed {
                                                self.chat_history.push((
                                                    parsed_author.clone(),
                                                    parsed_content.clone(),
                                                ));
                                            } else {
                                                *self
                                                    .unread_counts
                                                    .entry(t_id.clone())
                                                    .or_insert(0) += 1;
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
                                        if !group.members.contains(&sender_id) {
                                            return Command::none();
                                        }
                                        let caller_pseudo = vd.contacts.get(&sender_id)
                                            .cloned()
                                            .unwrap_or_else(|| "Inconnu".to_string());
                                        
                                        self.incoming_call = Some((sender_id.clone(), caller_pseudo, "".to_string(), grp_id.clone()));
                                        self.incoming_call_timer = 60;
                                        self.status_message = crate::ui::i18n::t(&self.language, "status_incoming_call");
                                        crate::sound::SOUND_MANAGER.lock().unwrap_or_else(|e| e.into_inner()).start_incoming_call();
                                        return Command::none();
                                    }
                                }
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:GROUP_SYNC:") {
                        let sys_parts: Vec<&str> = text.splitn(5, ':').collect();

                        if sys_parts.len() == 5 {
                            let grp_id = sys_parts[2].trim().chars().take(100).collect::<String>();
                            let grp_name = sys_parts[3].trim().chars().take(50).collect::<String>();
                            let members: Vec<String> = sys_parts[4]
                                .split(',')
                                .take(100)
                                .map(|s| s.trim().chars().take(100).collect::<String>())
                                .collect();

                            if let (Some(vd), Some(_pwd)) =
                                (&mut self.vault_data, &self.master_password)
                            {
                                if vd.tombstones.contains(&grp_id) {
                                    return Command::none();
                                }
                                let is_new = !vd.groups.contains_key(&grp_id);
                                if is_new && !vd.contacts.contains_key(&sender_id) {
                                    return Command::none();
                                }
                                const MAX_GROUPS: usize = 200;
                                if is_new && vd.groups.len() >= MAX_GROUPS {
                                    return Command::none();
                                }
                                let mut authorized = is_new;
                                if !is_new {
                                    if let Some(existing) = vd.groups.get(&grp_id) {
                                        if existing.members.contains(&sender_id) {
                                            if existing.members.first() == members.first() {
                                                authorized = true;
                                            }
                                        }
                                    }
                                }

                                if authorized {
                                    let mut changed = true;
                                    if let Some(existing) = vd.groups.get(&grp_id) {
                                        if existing.name == grp_name && existing.members == members
                                        {
                                            changed = false;
                                        }
                                    }

                                    if changed {
                                        vd.groups.insert(
                                            grp_id.clone(),
                                            crate::crypto::GroupData {
                                                name: grp_name.clone(),
                                                members,
                                            },
                                        );
                                        self.needs_save = true;
                                    }

                                    if is_new {
                                        self.status_message = crate::ui::i18n::t(
                                            &self.language,
                                            "status_server_invited",
                                        )
                                        .replace("{name}", &grp_name);
                                    }
                                }
                            }
                        }
                        return Command::none();
                    }

                    if text.starts_with("SYS:GROUP_LEAVE:") {
                        let sys_parts: Vec<&str> = text.splitn(3, ':').collect();

                        if sys_parts.len() == 3 {
                            let grp_id = sys_parts[2].trim().to_string();

                            if let (Some(vd), Some(_pwd)) =
                                (&mut self.vault_data, &self.master_password)
                            {
                                if let Some(group) = vd.groups.get_mut(&grp_id) {
                                    group.members.retain(|m| m.trim() != sender_id);
                                    self.needs_save = true;
                                    self.status_message =
                                        crate::ui::i18n::t(&self.language, "status_member_left");
                                }
                            }
                        }
                        return Command::none();
                    }

                    let mut target_chat_id = sender_id.clone();
                    let mut display_text = text.clone();

                                        if text.starts_with("SYS:GRP_MSG:") {
                        let sys_parts: Vec<&str> = text.splitn(6, ':').collect();
                        if sys_parts.len() == 6 && sys_parts[3] == "v2" {
                            let possible_target = sys_parts[2].trim().to_string();
                            let mut authorized = false;
                            let mut my_id = String::new();
                            if let Some(vd) = &self.vault_data {
                                my_id = crate::crypto::derive_public_id(&vd.private_key);
                                if let Some(group) = vd.groups.get(&possible_target) {
                                    if group.members.contains(&sender_id) { authorized = true; }
                                }
                            }
                            if !authorized { return Command::none(); }
                            let (sig, body) = sys_parts[5].split_once(':').unwrap_or(("", ""));
                            let ts_str = sys_parts[4].trim();
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                            let mut sig_valid = false;
                            if let Ok(ts) = ts_str.parse::<u64>() {
                                let fresh = ts.abs_diff(now) <= 60;
                                if fresh && crate::crypto::verify_message(&sender_id, &crate::crypto::conv_id(&my_id, &possible_target), ts, body, sig) {
                                    sig_valid = true;
                                }
                            }
                            if !sig_valid { return Command::none(); }
                            target_chat_id = possible_target;
                            display_text = body.to_string();
                        } else {
                            let p: Vec<&str> = text.splitn(4, ':').collect();
                            if p.len() == 4 {
                                let possible_target = p[2].trim().to_string();
                                let mut authorized = false;
                                if let Some(vd) = &self.vault_data {
                                    if let Some(group) = vd.groups.get(&possible_target) {
                                        if group.members.contains(&sender_id) { authorized = true; }
                                    }
                                }
                                if !authorized { return Command::none(); }
                                target_chat_id = possible_target;
                                let (_, body) = p[3].split_once(':').unwrap_or(("", p[3]));
                                display_text = body.to_string();
                            }
                        }
                    }

                    let sender_pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts
                            .get(&sender_id)
                            .cloned()
                            .unwrap_or_else(|| "Inconnu".to_string())
                    } else {
                        "Inconnu".to_string()
                    };

                    if let (Some(vd), Some(_pwd)) = (&mut self.vault_data, &self.master_password) {
                        let entry = crypto::MessageEntry {
                            author: sender_pseudo.clone(),
                            content: display_text.clone(),
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                            is_media: false,
                            media_key: None,
                            media_path: None,
                            signature: None,
                        };

                        // EVENT SOURCING: Auto-cicatrisation des dparts en STR (Temps Rel)
                        if display_text.starts_with("SYS:EVT:LEAVE:") {
                            let payload = display_text.trim_start_matches("SYS:EVT:LEAVE:").trim();
                            let parts: Vec<&str> = payload.splitn(2, ':').collect();
                            if parts.len() == 2 {
                                let left_id = parts[0];
                                let ts_str = parts[1];
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs();
                                let fresh = ts_str
                                    .parse::<u64>()
                                    .map(|ts| ts >= now.saturating_sub(60) && ts <= now + 60)
                                    .unwrap_or(false);
                                if fresh && left_id.to_lowercase() == sender_id.to_lowercase() {
                                    if let Some(group) = vd.groups.get_mut(&target_chat_id) {
                                        group.members.retain(|m| {
                                            m.trim().to_lowercase() != left_id.to_lowercase()
                                        });
                                    }
                                }
                            }
                        }

                        vd.chat_history
                            .entry(target_chat_id.clone())
                            .or_default()
                            .push(entry);
                        self.needs_save = true;
                    }

                    let is_currently_viewed = self.selected_chat.as_ref() == Some(&target_chat_id);

                    SOUND_MANAGER
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .play_message_received();
                    if is_currently_viewed {
                        self.chat_history.push((sender_pseudo, display_text));
                    } else {
                        *self
                            .unread_counts
                            .entry(target_chat_id.clone())
                            .or_insert(0) += 1;
                    }
                }
            } else if msg.starts_with("CONTACT:") {
                let parts: Vec<&str> = msg.splitn(3, ':').collect();

                if parts.len() == 3 {
                    let c_id = parts[1].trim().to_string();
                    let c_pseudo = parts[2].trim().to_string();

                    if let (Some(vd), Some(_pwd)) = (&mut self.vault_data, &self.master_password) {
                        if !vd.contacts.contains_key(&c_id) {
                            if !vd.pending_requests.contains_key(&c_id) {
                                const MAX_PENDING: usize = 100;
                                let c_pseudo_clean: String = c_pseudo.chars().filter(|c| !c.is_control()).take(32).collect();
                                if vd.pending_requests.len() >= MAX_PENDING { return Command::none(); }
                                vd.pending_requests.insert(c_id.clone(), c_pseudo_clean.clone());
                                self.needs_save = true;
                                self.status_message =
                                    crate::ui::i18n::t(&self.language, "status_new_request");
                            } else {
                                // Mettre   jour le pseudo de la demande en attente
                                let c_pseudo_clean: String = c_pseudo.chars().filter(|c| !c.is_control()).take(32).collect();
                                vd.pending_requests.insert(c_id, c_pseudo_clean);
                            }
                        } else {
                            // C'est un de nos contacts. Mettons   jour son pseudo officiel s'il tait inconnu !
                            let c_pseudo_clean: String = c_pseudo.chars().filter(|c| !c.is_control()).take(32).collect();
                            vd.contacts.insert(c_id, c_pseudo_clean);
                            self.needs_save = true;
                        }
                    }
                }
            } else if msg.starts_with("FILE_OFFER|") {
                let parts: Vec<&str> = msg.splitn(5, '|').collect();
                if parts.len() == 5 {
                    let sender_id = parts[1].to_string();
                    if let Some(vd) = &self.vault_data {
                        if !vd.contacts.contains_key(&sender_id) || vd.blocked_ids.contains(&sender_id) {
                            return Command::none();
                        }
                    }
                    let total: usize = parts[2].parse().unwrap_or(0);
                    let key_b64 = parts[3].to_string();
                    let filename = parts[4].to_string();
                    if total > 0 {
                        if filename.starts_with("SYNC|") {
                            // [LOW-1] Anti-spam d'historique
                            let mut authorized = false;
                            if let Some(req_time) = self.sync_requests.get(&sender_id) {
                                if req_time.elapsed().as_secs() < 120 {
                                    authorized = true;
                                }
                            }
                            if authorized {
                                let _ = self.tx_network.send(format!(
                                    "ACCEPT_FILE|{}|{}|{}|{}",
                                    sender_id, total, key_b64, filename
                                ));
                            } else {
                                self.status_message = crate::ui::i18n::t(&self.language, "status_spam_rejected");
                            }
                        } else {
                            self.incoming_file_offers
                                .retain(|(s, f, _, _)| s != &sender_id || f != &filename);
                            if self.incoming_file_offers.len() < 10 {
                                self.incoming_file_offers
                                    .push((sender_id, filename, total, key_b64));
                                SOUND_MANAGER
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .play_message_received();
                            }
                        }
                    }
                }
            } else if msg.starts_with("INCOMING_CALL:") {
                let parts: Vec<&str> = msg.splitn(4, ':').collect();

                if parts.len() == 4 {
                    let caller_id = parts[1].trim().to_string();
                    let caller_grp_id = parts[2].trim().to_string();
                    let sdp = parts[3].to_string();

                    // --- [MED-4] Anti-Spam / Ligne Occupée ---
                    if let Some((active_id, _)) = &self.active_call {
                        if active_id != &caller_grp_id && active_id != &caller_id {
                            // BLOCK if we are P2P and someone else calls us, OR we are in a group and someone OUTSIDE the group calls us.
                            // However, we MUST NOT block if it's a group call and someone from the same group is calling us.
                            // But wait! If `caller_grp_id` is empty, someone is calling us P2P!
                            // If `caller_grp_id` is empty, AND `caller_id` != `active_id`, we MUST REJECT!
                            if caller_grp_id.is_empty() || caller_grp_id != *active_id {
                                // [MED-4] Auto-Signal au correspondant pour qu'il ne poireaute pas 25s
                                let tx = self.tx_network.clone();
                                let cid = caller_id.clone();
                                tokio::spawn(async move {
                                    let _ = tx.send(format!("CHAT_SEND:{}:SYS:CALL_BUSY", cid));
                                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                                    let _ = tx.send(format!("REJECT:{}", cid));
                                });
                                if let (Some(vd), Some(_pwd)) =
                                    (&mut self.vault_data, &self.master_password)
                                {
                                    let entry = crate::crypto::MessageEntry {
                                        author: crate::ui::i18n::t(&self.language, "system_author"),
                                        content: crate::ui::i18n::t(
                                            &self.language,
                                            "msg_missed_call",
                                        ),
                                        timestamp: std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap()
                                            .as_secs(),
                                        is_media: false,
                                        media_key: None,
                                        media_path: None,
                                        signature: None,
                                    };
                                    vd.chat_history
                                        .entry(caller_id.clone())
                                        .or_default()
                                        .push(entry);
                                    self.needs_save = true;
                                }
                                return iced::Task::none(); // On ignore silencieusement l'appel à l'écran
                            }
                        }
                    }

                    // Old flawed auto-accept logic is removed.
                    if let Some((active_id, _)) = &self.active_call {
                        if active_id.starts_with("grp_") {
                            if let Some(vd) = &self.vault_data {
                                if let Some(group) = vd.groups.get(active_id) {
                                    if group.members.contains(&caller_id) {
                                        let _ = self
                                            .tx_network
                                            .send(format!("ACCEPT:{}:{}", caller_id, sdp));
                                        return Command::none();
                                    }
                                }
                            }
                        }
                    }

                    let mut caller_pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts
                            .get(&caller_id)
                            .cloned()
                            .unwrap_or_else(|| "Inconnu".to_string())
                    } else {
                        "Inconnu".to_string()
                    };

                    if !caller_grp_id.is_empty() {
                        if let Some(vd) = &self.vault_data {
                            if let Some(grp) = vd.groups.get(&caller_grp_id) {
                                caller_pseudo = grp.name.clone();
                            }
                        }
                    }

                    if let Some((_, _, _, inc_grp)) = &self.incoming_call {
                        if !inc_grp.is_empty() && inc_grp == &caller_grp_id {
                            self.queued_group_offers
                                .push((caller_id.clone(), sdp.clone()));
                            return Command::none();
                        }
                    }

                    self.incoming_call_timer = 0;
                    self.incoming_call = Some((caller_id, caller_pseudo, sdp, caller_grp_id));
                    SOUND_MANAGER
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .start_incoming_call();
                }
            } else if msg.starts_with("CALL_ACTIVE:") {
                let id = msg.trim_start_matches("CALL_ACTIVE:").trim().to_string();

                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .stop_outgoing_call();
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .stop_incoming_call();
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .play_call_connected();
                if self.status_message.starts_with("LOADING:") {
                    self.status_message.clear();
                }

                // Si on a pas d'appel actif, c'est nous qui avons initié l'appel P2P !
                if self.active_call.is_none() {
                    let pseudo = if let Some(vd) = &self.vault_data {
                        vd.contacts
                            .get(&id)
                            .cloned()
                            .unwrap_or_else(|| "Ami".to_string())
                    } else {
                        "Ami".to_string()
                    };

                    self.active_call = Some((id.clone(), pseudo));
                    self.active_call_participants.clear(); // Vider les fantomes !
                    self.active_call_participants.insert(id.clone()); // Ajouter notre correspondant

                    self.call_start_time = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                    );
                    self.chat_input.clear();
                    self.chat_history.clear();

                    if let Some(vd) = &self.vault_data {
                        if let Some(history) = vd.chat_history.get(&id) {
                            for msg in history {
                                self.chat_history
                                    .push((msg.author.clone(), msg.content.clone()));
                            }
                        }
                    }
                    return self.trigger_history_sync(&id);
                } else {
                    // Un appel est déjà actif (Groupe ou P2P)
                    if let Some((active_id, _)) = &self.active_call {
                        if active_id.starts_with("grp_") || active_id == &id {
                            self.active_call_participants.insert(id.clone());
                        }

                        if active_id.starts_with("grp_") {
                            let _ = self
                                .tx_network
                                .send(format!("CHAT_SEND:{}:SYS:CALL_CONTEXT:{}", id, active_id));
                            return Command::none();
                        }
                    }
                }
            } else if msg.starts_with("ERROR:NOT_FOUND:") {
                let tgt = msg.trim_start_matches("ERROR:NOT_FOUND:");
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .stop_outgoing_call();
                self.status_message =
                    crate::ui::i18n::t(&self.language, "status_remote_unavailable");
                let _ = self.tx_network.send(format!("HANGUP:{}", tgt));

                if let Some(start) = self.call_start_time.take() {
                    let now_s = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
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
                        vd.chat_history
                            .entry(tgt.to_string())
                            .or_default()
                            .push(entry);
                        self.needs_save = true;
                    }
                }
                self.active_call = None;
            } else if msg.starts_with("CALL_BUSY:") {
                let id = msg.trim_start_matches("CALL_BUSY:").trim().to_string();
                SOUND_MANAGER
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .stop_outgoing_call();
                let _ = self.tx_network.send(format!("HANGUP:{}", id));

                let mut pseudo = "L'interlocuteur".to_string();
                if let Some(vd) = &self.vault_data {
                    if let Some(contact_pseudo) = vd.contacts.get(&id) {
                        pseudo = contact_pseudo.clone();
                    }
                }

                self.status_message =
                    crate::ui::i18n::t(&self.language, "status_remote_busy_named")
                        .replace("{name}", &pseudo);

                if let Some((active_id, _)) = &self.active_call {
                    if active_id == &id {
                        self.active_call = None;
                        self.is_muted = false;
                        let _ = self.tx_network.send("MUTE:off".to_string());
                        self.chat_input.clear();
                        self.chat_history.clear();
                    }
                }
            } else if msg.starts_with("CALL_CONNECTED:") {
                let id = msg.trim_start_matches("CALL_CONNECTED:").trim().to_string();
                let mut allow = false;
                if let Some((act_id, _)) = &self.active_call {
                    if act_id.starts_with("grp_") || act_id == &id {
                        allow = true;
                    }
                }
                if allow {
                    self.active_call_participants.insert(id);
                }
                return iced::Task::none();
            } else if msg.starts_with("CALL_ENDED:") {
                let id = msg.trim_start_matches("CALL_ENDED:").trim().to_string();
                self.active_call_participants.remove(&id);
                let _ = self.tx_network.send(format!("HANGUP:{}", id));

                let mut should_end = false;

                if let Some((active_id, _)) = &self.active_call {
                    if active_id == &id {
                        should_end = true;
                    }
                }

                if should_end {
                    self.active_call = None;
                    SOUND_MANAGER
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .play_call_disconnected();
                    self.is_muted = false;
                    let _ = self.tx_network.send("MUTE:off".to_string());

                    self.chat_input.clear();
                    self.chat_history.clear();
                    self.status_message =
                        crate::ui::i18n::t(&self.language, "status_remote_hangup");
                }

                if let Some((inc_id, _, _, _)) = &self.incoming_call {
                    if inc_id == &id {
                        SOUND_MANAGER
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .stop_incoming_call();
                        self.incoming_call = None;
                        self.incoming_call_timer = 0;
                        if !should_end {
                            self.status_message =
                                crate::ui::i18n::t(&self.language, "status_caller_hangup");
                        }
                    }
                }
            } else if msg.starts_with("TIMEOUT:") {
                let tgt = msg.trim_start_matches("TIMEOUT:");
                let is_current = if let Some((active_id, _)) = &self.active_call {
                    active_id == tgt
                } else {
                    false
                };

                if is_current || self.active_call.is_none() {
                    if self.status_message.starts_with("LOADING:")
                        || self.status_message.contains("attente")
                    {
                        SOUND_MANAGER
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .stop_outgoing_call();
                        self.status_message =
                            crate::ui::i18n::t(&self.language, "status_remote_unavailable");
                        let _ = self.tx_network.send(format!("HANGUP:{}", tgt));
                        self.active_call = None;
                    }
                }
            } else if msg.starts_with("LOADING:") {
                // Ignore late loading spams if call is already connected or failed (Anti-rebond UI)
                if self.active_call.is_none() {
                    if self.status_message.starts_with("LOADING:")
                        || self.status_message.contains("attente")
                    {
                        self.status_message = msg;
                    }
                }
            } else {
                self.status_message = msg;
            }
        }
        Command::none()
    }
}

