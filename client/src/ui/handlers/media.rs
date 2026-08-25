use iced::Command;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn handle_media(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::OpenFileDialog => {
                self.idle_seconds = 0;
                return Command::perform(async {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Sélectionner un fichier (Max 50 Mo)")
                        .add_filter("Fichiers sécurisés", &["jpg", "png", "rar", "zip", "pdf", "docx", "txt"])
                        .pick_file()
                        .await;
                    file.map(|f| f.path().to_string_lossy().to_string())
                }, Message::FileSelected);
            }
            Message::FileSelected(path_opt) => {
                self.idle_seconds = 0;
                if let Some(path) = path_opt {
                    return Command::perform(async move {
                        if let Ok(metadata) = tokio::fs::metadata(&path).await {
                            if metadata.len() > 52_428_800 {
                                return Some(("ERROR_SIZE".to_string(), vec![]));
                            }
                        }
                        if let Ok(raw_data) = tokio::fs::read(&path).await {
                            let file_name = std::path::Path::new(&path).file_name().unwrap_or_default().to_string_lossy().into_owned();
                            Some((file_name, raw_data))
                        } else { None }
                    }, Message::FileRead);
                }
            }
            Message::FileRead(data_opt) => {
                self.idle_seconds = 0;
                if let Some((file_name, raw_data)) = data_opt {
                    if file_name == "ERROR_SIZE" {
                        self.status_message = "❌ Erreur : Le fichier dépasse la limite de 50 Mo.".to_string();
                        return Command::none();
                    }

                    let target = if let Some((active_id, _)) = &self.active_call { Some(active_id.clone()) } else { self.selected_chat.clone() };

                    if let Some(target_id) = target {
                        if let (Some(vd), Some(pwd)) = (&mut self.vault_data, &self.master_password) {
                            if let Ok((key_bytes, enc_path)) = crate::crypto::encrypt_and_save_media(&vd.pseudo, &file_name, &raw_data) {
                                use base64::prelude::*;
                                let key_b64 = BASE64_STANDARD.encode(key_bytes);
                                let my_id = crate::crypto::derive_public_id(&vd.private_key);

                                let mut broadcast_cmd = Command::none();

                                if target_id.starts_with("grp_") {
                                    if let Some(group) = vd.groups.get(&target_id) {
                                        let net_filename = format!("{}|{}", target_id, file_name.clone());
                                        let tx = self.tx_network.clone();
                                        let members = group.members.clone();
                                        let enc_path_net = enc_path.clone();

                                        broadcast_cmd = Command::perform(
                                            async move {
                                                for member_id in members {
                                                    if member_id != my_id {
                                                        let _ = tx.send(format!("FILE_SEND_INIT:{}:{}:{}:{}", member_id, net_filename, key_b64, enc_path_net));
                                                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                                                    }
                                                }
                                            },
                                            |_| Message::ResetInactivity
                                        );
                                    }
                                } else {
                                    let _ = self.tx_network.send(format!("FILE_SEND_INIT:{}:{}:{}:{}", target_id, file_name.clone(), key_b64, enc_path.clone()));
                                }

                                let entry = crate::crypto::MessageEntry {
                                    author: "Moi".to_string(),
                                    content: format!("📎 Fichier partagé : {}", file_name),
                                    timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
                                    is_media: true,
                                    media_key: Some(key_bytes),
                                    media_path: Some(enc_path),
                                };
                                vd.chat_history.entry(target_id.clone()).or_default().push(entry);
                                let _ = crate::crypto::save_vault(pwd, vd);

                                self.chat_history.push(("Moi".to_string(), format!("📎 Fichier partagé : {}", file_name)));

                                return broadcast_cmd;
                            }
                        }
                    }
                }
            }
            Message::OpenMedia(filename, key, path) => {
                self.idle_seconds = 0;
                return Command::perform(async move {
                    let clean_name = filename.replace("📎 Fichier reçu : ", "").replace("📎 Fichier partagé : ", "");
                    let dest = rfd::AsyncFileDialog::new().set_title("Extraction...").set_file_name(&clean_name).save_file().await;

                    if let Some(dest_path) = dest {
                        let dest_path_str = dest_path.path().to_string_lossy().to_string();

                        // --- ÉTAPE 2 : MULTITHREADING SUR L'EXTRACTION ---
                        let decrypted_data_res = tokio::task::spawn_blocking(move || {
                            crate::crypto::decrypt_media(&path, &key)
                        }).await.unwrap();

                        if let Ok(decrypted_data) = decrypted_data_res {
                            let _ = tokio::fs::write(&dest_path_str, decrypted_data).await;
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("cmd").args(["/c", "start", "", &dest_path_str]).spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(&dest_path_str).spawn();
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open").arg(&dest_path_str).spawn();
                            format!("✅ Fichier déchiffré !")
                        } else { "❌ Erreur de déchiffrement.".to_string() }
                    } else { "⚠️ Extraction annulée.".to_string() }
                }, Message::MediaSaved);
            }
            Message::MediaSaved(msg) => {
                self.idle_seconds = 0;
                self.status_message = msg;
            }

            // --- ÉTAPES 1 & 2 : APERÇU RAM MULTITHREADÉ ---
            Message::PreviewMedia(path, key) => {
                self.idle_seconds = 0;
                self.status_message = "⏳ Chargement sécurisé de l'aperçu...".to_string();

                return Command::perform(async move {
                    // La cryptographie lourde est envoyée sur un autre thread pour libérer l'UI
                    let res = tokio::task::spawn_blocking(move || {
                        crate::crypto::decrypt_media(&path, &key)
                    }).await.unwrap();
                    res.ok() // Retourne Option<Vec<u8>>
                }, Message::PreviewMediaLoaded);
            }
            Message::PreviewMediaLoaded(data_opt) => {
                self.idle_seconds = 0;
                if let Some(decrypted_bytes) = data_opt {
                    self.media_preview = Some(iced::widget::image::Handle::from_memory(decrypted_bytes));
                    self.status_message = "✅ Aperçu média chargé en mémoire RAM (Zéro-Trace).".to_string();
                } else {
                    self.status_message = "❌ Impossible de générer l'aperçu.".to_string();
                }
            }
            Message::ClosePreview => {
                self.idle_seconds = 0;
                self.media_preview = None;
                self.status_message = "✅ Aperçu fermé (Données purgées de la RAM).".to_string();
            }

            _ => {}
        }
        Command::none()
    }
}