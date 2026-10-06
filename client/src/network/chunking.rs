use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use webrtc::data_channel::RTCDataChannel;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const CHUNK_SIZE: usize = 16384;

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct OutgoingTransfer {
    pub file_data: Vec<u8>,
    #[zeroize(skip)]
    pub total_chunks: usize,
}

// Removed Clone to allow std::fs::File, skip zeroize on it
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct IncomingTransfer {
    #[zeroize(skip)]
    pub key_b64: String,
    #[zeroize(skip)]
    pub total_chunks: usize,
    #[zeroize(skip)]
    pub received_chunks: usize,
    #[zeroize(skip)]
    pub temp_path: String,
    #[zeroize(skip)]
    pub file_handle: Option<std::fs::File>,
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

    pub fn cleanup(&mut self, target_id: &str) {
        let prefix = format!("{}_", target_id);
        self.outgoing.retain(|k, _| !k.starts_with(&prefix));

        let mut to_remove = Vec::new();
        let target_pattern = format!("_{}_", target_id);
        for k in self.incoming.keys() {
            if k.contains(&target_pattern) {
                to_remove.push(k.clone());
            }
        }
        for k in to_remove {
            if let Some(transfer) = self.incoming.remove(&k) {
                let _ = std::fs::remove_file(&transfer.temp_path);
            }
        }
    }

    pub async fn handle_message(
        &mut self,
        sender_id: &str,
        raw_payload: &[u8],
        data_channels: &HashMap<String, Arc<RTCDataChannel>>,
        tx_ui: &UnboundedSender<String>,
        my_pseudo: &str,
    ) {
        let parts: Vec<&[u8]> = raw_payload.splitn(5, |&b| b == b':').collect();
        if parts.len() < 2 || parts[0] != b"SYS" {
            return;
        }

        let sys_cmd = std::str::from_utf8(parts[1]).unwrap_or("");

        if sys_cmd == "FILE_META" && parts.len() == 5 {
            let filename = std::str::from_utf8(parts[2]).unwrap_or("").to_string();
            let total: usize = std::str::from_utf8(parts[3])
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);

            // SECURITY FIX: HARD LIMIT ON CHUNKS (1 GB max)
            if total > 3202 {
                return;
            }

            let key_b64 = std::str::from_utf8(parts[4]).unwrap_or("").to_string();
            let _ = tx_ui.send(format!(
                "FILE_OFFER|{}|{}|{}|{}",
                sender_id, total, key_b64, filename
            ));
        } else if sys_cmd == "ACK_META" && parts.len() >= 3 {
            let filename = std::str::from_utf8(parts[2]).unwrap_or("");
            let transfer_key = format!("{}_{}", sender_id, filename);

            if let Some(mut transfer) = self.outgoing.remove(&transfer_key) {
                if let Some(dc) = data_channels.get(sender_id).cloned() {
                    let filename_owned = filename.to_string();
                    tokio::spawn(async move {
                        for index in 0..transfer.total_chunks {
                            let start = index * CHUNK_SIZE;
                            let end = std::cmp::min(start + CHUNK_SIZE, transfer.file_data.len());

                            let mut payload =
                                format!("SYS:FILE_CHUNK:{}:{}:", filename_owned, index)
                                    .into_bytes();
                            payload.extend_from_slice(&transfer.file_data[start..end]);

                            if let Err(_) = dc.send(&bytes::Bytes::from(payload)).await {
                                break;
                            }
                            while dc.buffered_amount().await > 1024 * 1024 {
                                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                            }
                        }
                        transfer.file_data.zeroize();
                    });
                }
            }
        } else if sys_cmd == "FILE_CHUNK" && parts.len() == 5 {
            let filename = std::str::from_utf8(parts[2]).unwrap_or("");
            let index: usize = std::str::from_utf8(parts[3])
                .unwrap_or("0")
                .parse()
                .unwrap_or(0);
            let chunk_data = parts[4];

            let file_id = format!("{}_{}_{}", my_pseudo, sender_id, filename);
            let mut transfer_done = false;

            if let Some(transfer) = self.incoming.get_mut(&file_id) {
                // SECURITY FIX: Validate chunk index strictly
                if index < transfer.total_chunks
                    && index == transfer.received_chunks
                    && chunk_data.len() <= CHUNK_SIZE
                {
                    // SECURITY FIX: STREAM TO DISK
                    if let Some(file) = &mut transfer.file_handle {
                        use std::io::Write;
                        let _ = file.write_all(chunk_data);
                    }

                    transfer.received_chunks += 1;
                    if transfer.received_chunks == transfer.total_chunks {
                        if let Some(mut f) = transfer.file_handle.take() {
                            let _ = f.sync_all(); // Flush disk guarantees
                        }
                    }
                    if transfer.received_chunks == transfer.total_chunks {
                        transfer_done = true;
                    }
                }
            }

            if transfer_done {
                if let Some(transfer) = self.incoming.remove(&file_id) {
                    let key_owned = transfer.key_b64.clone();
                    let sender_owned = sender_id.to_string();
                    let filename_owned = filename.to_string();
                    let tx_ui_clone = tx_ui.clone();
                    let pseudo_clone = my_pseudo.to_string();
                    let temp_path_owned = transfer.temp_path.clone();

                    tokio::spawn(async move {
                        let media_dir = crate::crypto::get_media_dir(&pseudo_clone);
                        // SECURITY FIX: NOUV-1 Path Traversal

                        let mut path_buf = std::path::PathBuf::from(media_dir);
                        path_buf.push(format!("kako_media_{}.dat", rand::random::<u64>()));
                        let save_path = path_buf.to_string_lossy().to_string();

                        // Move/Rename file from temp_path to save_path
                        if tokio::fs::rename(&temp_path_owned, &save_path)
                            .await
                            .is_ok()
                        {
                            let _ = tx_ui_clone.send(format!(
                                "FILE_RECV:{}|{}|{}|{}",
                                sender_owned, key_owned, save_path, filename_owned
                            ));
                        } else {
                            // Fallback to copy+remove if across file systems
                            if tokio::fs::copy(&temp_path_owned, &save_path).await.is_ok() {
                                let _ = tokio::fs::remove_file(&temp_path_owned).await;
                                let _ = tx_ui_clone.send(format!(
                                    "FILE_RECV:{}|{}|{}|{}",
                                      sender_owned, key_owned, save_path, filename_owned
                                ));
                            }
                        }
                    });
                }
            }
        }
    }
}
