use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use webrtc::data_channel::RTCDataChannel;
use base64::prelude::*;

pub const CHUNK_SIZE: usize = 8192;

#[derive(Clone)]
pub struct OutgoingTransfer {
    pub file_data: Vec<u8>,
    pub total_chunks: usize,
}

#[derive(Clone)]
pub struct IncomingTransfer {
    pub key_b64: String,
    pub total_chunks: usize,
    pub received_chunks: usize,
    pub chunks: Vec<String>,
}

pub struct TransferManager {
    pub outgoing: HashMap<String, OutgoingTransfer>,
    pub incoming: HashMap<String, IncomingTransfer>,
}

impl TransferManager {
    pub fn new() -> Self {
        Self {
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
        }
    }

    pub async fn handle_message(
        &mut self,
        sender_id: &str,
        text: &str,
        data_channels: &HashMap<String, Arc<RTCDataChannel>>,
        tx_ui: &UnboundedSender<String>,
        my_pseudo: &str,
    ) {
        if text.starts_with("SYS:FILE_META:") {
            let parts: Vec<&str> = text.splitn(5, ':').collect();
            if parts.len() == 5 {
                let filename = parts[2].to_string();
                let total: usize = parts[3].parse().unwrap_or(0);
                let key_b64 = parts[4].to_string();

                let file_id = format!("{}_{}", sender_id, filename);
                self.incoming.insert(file_id, IncomingTransfer {
                    key_b64,
                    total_chunks: total,
                    received_chunks: 0,
                    chunks: vec![String::new(); total],
                });

                if let Some(dc) = data_channels.get(sender_id) {
                    let _ = dc.send_text(format!("SYS:ACK_META:{}", filename)).await;
                }
            }
        }
        else if text.starts_with("SYS:ACK_META:") {
            let filename = text.trim_start_matches("SYS:ACK_META:");
            let transfer_key = format!("{}_{}", sender_id, filename);

            if let Some(transfer) = self.outgoing.get(&transfer_key) {
                let end = std::cmp::min(CHUNK_SIZE, transfer.file_data.len());
                let b64 = BASE64_STANDARD.encode(&transfer.file_data[0..end]);

                if let Some(dc) = data_channels.get(sender_id) {
                    let _ = dc.send_text(format!("SYS:FILE_CHUNK:{}:0:{}", filename, b64)).await;
                }
            }
        }
        else if text.starts_with("SYS:FILE_CHUNK:") {
            let parts: Vec<&str> = text.splitn(5, ':').collect();
            if parts.len() == 5 {
                let filename = parts[2];
                let index: usize = parts[3].parse().unwrap_or(0);
                let b64_data = parts[4];

                let file_id = format!("{}_{}", sender_id, filename);
                if let Some(transfer) = self.incoming.get_mut(&file_id) {
                    if index < transfer.total_chunks && transfer.chunks[index].is_empty() {
                        transfer.chunks[index] = b64_data.to_string();
                        transfer.received_chunks += 1;

                        if let Some(dc) = data_channels.get(sender_id) {
                            let _ = dc.send_text(format!("SYS:ACK_CHUNK:{}:{}", filename, index)).await;
                        }

                        if transfer.received_chunks == transfer.total_chunks {
                            let chunks_owned = std::mem::take(&mut transfer.chunks);
                            let filename_owned = filename.to_string();
                            let key_owned = transfer.key_b64.clone();
                            let sender_owned = sender_id.to_string();
                            let tx_ui_clone = tx_ui.clone();
                            let pseudo_clone = my_pseudo.to_string();

                            tokio::spawn(async move {
                                let mut full_data = Vec::new();
                                for c in chunks_owned {
                                    if let Ok(bytes) = BASE64_STANDARD.decode(&c) {
                                        full_data.extend(bytes);
                                    }
                                }
                                let media_dir = format!("media_{}", pseudo_clone);
                                let _ = tokio::fs::create_dir_all(&media_dir).await;

                                let safe_filename = filename_owned.replace('|', "_");
                                let save_path = format!("{}/{}", media_dir, safe_filename);

                                if tokio::fs::write(&save_path, full_data).await.is_ok() {
                                    let _ = tx_ui_clone.send(format!("FILE_RECV:{}:{}:{}:{}", sender_owned, filename_owned, key_owned, save_path));
                                }
                            });
                            self.incoming.remove(&file_id);
                        }
                    }
                }
            }
        }
        else if text.starts_with("SYS:ACK_CHUNK:") {
            let parts: Vec<&str> = text.splitn(4, ':').collect();
            if parts.len() == 4 {
                let filename = parts[2];
                let ack_index: usize = parts[3].parse().unwrap_or(0);
                let transfer_key = format!("{}_{}", sender_id, filename);

                if let Some(transfer) = self.outgoing.get_mut(&transfer_key) {
                    let next_index = ack_index + 1;
                    if next_index < transfer.total_chunks {
                        let start = next_index * CHUNK_SIZE;
                        if start < transfer.file_data.len() {
                            let end = std::cmp::min(start + CHUNK_SIZE, transfer.file_data.len());
                            let b64 = BASE64_STANDARD.encode(&transfer.file_data[start..end]);

                            if let Some(dc) = data_channels.get(sender_id) {
                                let _ = dc.send_text(format!("SYS:FILE_CHUNK:{}:{}:{}", filename, next_index, b64)).await;
                            }
                        }
                    } else {
                        self.outgoing.remove(&transfer_key);
                    }
                }
            }
        }
    }
}
