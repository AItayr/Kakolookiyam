use iced::Task as Command;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn trigger_global_sync(&self) -> Command<Message> {
        let tx = self.tx_network.clone();
        let mut sync_tasks = Vec::new();

        if let Some(vd) = &self.vault_data {
            let my_id = crate::crypto::derive_public_id(&vd.private_key);

            for contact_id in vd.contacts.keys() {
                let mut last_ts = 0;
                if let Some(history) = vd.chat_history.get(contact_id) {
                    if let Some(last_msg) = history.last() {
                        last_ts = last_msg.timestamp;
                    }
                }
                sync_tasks.push((contact_id.clone(), vec![contact_id.clone()], last_ts));
            }

            for (group_id, group_data) in &vd.groups {
                let mut last_ts = 0;
                if let Some(history) = vd.chat_history.get(group_id) {
                    if let Some(last_msg) = history.last() {
                        last_ts = last_msg.timestamp;
                    }
                }

                let mut peers = Vec::new();
                for member in &group_data.members {
                    if member != &my_id {
                        peers.push(member.clone());
                    }
                }

                if !peers.is_empty() {
                    sync_tasks.push((group_id.clone(), peers, last_ts));
                }
            }
        }

        if sync_tasks.is_empty() {
            return Command::none();
        }

        Command::perform(async move {
            for (target_id, peers, last_ts) in sync_tasks {
                let req_msg = format!("SYS:SYNC_REQ:{}:{}", target_id, last_ts);

                for peer_id in peers {
                    let _ = tx.send(format!("CHAT_SEND:{}:{}", peer_id, req_msg));
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                }
            }
        }, |_| Message::ResetInactivity)
    }

    pub(crate) fn trigger_history_sync(&self, target_id: &str) -> Command<Message> {
        let mut last_timestamp = 0;
        let mut peers_to_ask = Vec::new();

        if let Some(vd) = &self.vault_data {
            if let Some(history) = vd.chat_history.get(target_id) {
                if let Some(last_msg) = history.last() {
                    last_timestamp = last_msg.timestamp;
                }
            }

            if target_id.starts_with("grp_") {
                if let Some(group) = vd.groups.get(target_id) {
                    let my_id = crate::crypto::derive_public_id(&vd.private_key);
                    for member in &group.members {
                        if member != &my_id {
                            peers_to_ask.push(member.clone());
                        }
                    }
                }
            } else {
                peers_to_ask.push(target_id.to_string());
            }
        }

        if peers_to_ask.is_empty() {
            return Command::none();
        }

        let tx = self.tx_network.clone();
        let target_id_owned = target_id.to_string();

        Command::perform(async move {
            let req_msg = format!("SYS:SYNC_REQ:{}:{}", target_id_owned, last_timestamp);
            for peer_id in peers_to_ask {
                let _ = tx.send(format!("CHAT_SEND:{}:{}", peer_id, req_msg));
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        }, |_| Message::ResetInactivity)
    }

    pub(crate) fn handle_sync_request(&self, requester_id: String, target_id_for_requester: String, last_ts: u64) -> Command<Message> {
        if let Some(vd) = &self.vault_data {
            let is_group = target_id_for_requester.starts_with("grp_");
              let local_target_id = if is_group { target_id_for_requester.clone() } else { requester_id.clone() };
              
              if is_group && vd.tombstones.contains(&local_target_id) {
                  let tx = self.tx_network.clone();
                  let p_req = requester_id.clone();
                  let p_tgt = local_target_id.clone();
                  return Command::perform(async move {
                      let _ = tx.send(format!("CHAT_SEND:{}:SYS:GRP_DEL:{}", p_req, p_tgt));
                  }, |_| Message::ResetInactivity);
              }

            // [FIX] On profit de toute requete de synchro entrante pour "Pousser" nos groupes
              // vers l'utilisateur s'il est membre, au cas où il aurait été ajouté en étant hors-ligne !
              if !is_group && !vd.groups.is_empty() {
                  let tx_push = self.tx_network.clone();
                  let req_id = requester_id.clone();
                  let mut syncs = Vec::new();
                  for (g_id, g_data) in &vd.groups {
                      if g_data.members.contains(&requester_id) {
                          let m_str = g_data.members.join(",");
                          syncs.push(format!("CHAT_SEND:{}:SYS:GROUP_SYNC:{}:{}:{}", req_id, g_id, g_data.name, m_str));
                      }
                  }
                  if !syncs.is_empty() {
                      // Fire and forget pour eviter de bloquer
                      tokio::spawn(async move {
                          for s in syncs {
                              let _ = tx_push.send(s);
                              tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                          }
                      });
                  }
              }

              let is_authorized = if is_group {
                vd.groups.get(&local_target_id).map_or(false, |g| g.members.contains(&requester_id))
            } else {
                true
            };

            if !is_authorized {
                return Command::none();
            }

            if let Some(history) = vd.chat_history.get(&local_target_id) {
                let missing_messages: Vec<_> = history.iter()
                    .filter(|m| m.timestamp > last_ts)
                    .cloned()
                    .collect();

                if missing_messages.is_empty() {
                    return Command::none();
                }

                let tx = self.tx_network.clone();
                let my_pseudo = vd.pseudo.clone();

                return Command::perform(async move {
                    for msg in missing_messages {
                        use base64::prelude::*;
                        let safe_content = BASE64_STANDARD.encode(msg.content.as_bytes());
                        let actual_author = if msg.author == "Moi" || msg.author == "Me" || msg.author == "أنا" { my_pseudo.clone() } else { msg.author.clone() };

                        if msg.is_media {
                            if let (Some(key), Some(path)) = (msg.media_key, &msg.media_path) {
                                let key_b64 = BASE64_STANDARD.encode(key);
                                let sync_filename = format!("SYNC|{}|{}|{}|{}", target_id_for_requester, msg.timestamp, actual_author, safe_content);

                                let _ = tx.send(format!("FILE_SEND_INIT:{}:{}:{}:{}", requester_id, sync_filename, key_b64, path));
                                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                            }
                        } else {
                            let sync_payload = format!("SYS:SYNC_RES:{}:{}:TXT:{}|{}", target_id_for_requester, msg.timestamp, actual_author, safe_content);
                            let _ = tx.send(format!("CHAT_SEND:{}:{}", requester_id, sync_payload));
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        }
                    }
                }, |_| Message::ResetInactivity);
            }
        }
        Command::none()
    }
}