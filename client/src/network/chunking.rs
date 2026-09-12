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

#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct IncomingTransfer {
    #[zeroize(skip)]
    pub key_b64: String,
    #[zeroize(skip)]
    pub total_chunks: usize,
    #[zeroize(skip)]
    pub received_chunks: usize,
    pub file_buffer: Vec<u8>,
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
        raw_payload: &[u8],
        data_channels: &HashMap<String, Arc<RTCDataChannel>>,
        tx_ui: &UnboundedSender<String>,
        my_pseudo: &str,
    ) {
        let parts: Vec<&[u8]> = raw_payload.splitn(5, |&b| b == b':').collect();
        if parts.len() < 2 || parts[0] != b"SYS" { return; }
        
        let sys_cmd = std::str::from_utf8(parts[1]).unwrap_or("");

        if sys_cmd == "FILE_META" && parts.len() == 5 {
            let filename = std::str::from_utf8(parts[2]).unwrap_or("").to_string();
            let total: usize = std::str::from_utf8(parts[3]).unwrap_or("0").parse().unwrap_or(0);
            let key_b64 = std::str::from_utf8(parts[4]).unwrap_or("").to_string();

            let file_id = format!("{}_{}", sender_id, filename);
            self.incoming.insert(file_id, IncomingTransfer {
                key_b64,
                total_chunks: total,
                received_chunks: 0,
                file_buffer: Vec::with_capacity(total * CHUNK_SIZE),
            });

            if let Some(dc) = data_channels.get(sender_id) {
                let _ = dc.send_text(format!("SYS:ACK_META:{}", filename)).await;
            }
        }
        else if sys_cmd == "ACK_META" && parts.len() >= 3 {
             let filename = std::str::from_utf8(parts[2]).unwrap_or("");
             let transfer_key = format!("{}_{}", sender_id, filename);

             if let Some(transfer) = self.outgoing.get(&transfer_key) {
                 let end = std::cmp::min(CHUNK_SIZE, transfer.file_data.len());
                 let mut payload = format!("SYS:FILE_CHUNK:{}:0:", filename).into_bytes();
                 payload.extend_from_slice(&transfer.file_data[0..end]);

                 if let Some(dc) = data_channels.get(sender_id) {
                     let _ = dc.send(&bytes::Bytes::from(payload)).await;
                 }
             }
        }
        else if sys_cmd == "FILE_CHUNK" && parts.len() == 5 {
             let filename = std::str::from_utf8(parts[2]).unwrap_or("");
             let index: usize = std::str::from_utf8(parts[3]).unwrap_or("0").parse().unwrap_or(0);
             let chunk_data = parts[4]; 

             let file_id = format!("{}_{}", sender_id, filename);
             let mut transfer_done = false;
             if let Some(transfer) = self.incoming.get_mut(&file_id) {
                 if index < transfer.total_chunks {
                     transfer.file_buffer.extend_from_slice(chunk_data);
                     transfer.received_chunks += 1;

                     if let Some(dc) = data_channels.get(sender_id) {
                         let _ = dc.send_text(format!("SYS:ACK_CHUNK:{}:{}", filename, index)).await;
                     }

                     if transfer.received_chunks == transfer.total_chunks {
                         transfer_done = true;
                     }
                 }
             }
             if transfer_done {
                 if let Some(mut transfer) = self.incoming.remove(&file_id) {
                     let mut owned_buffer = std::mem::take(&mut transfer.file_buffer);
                     let key_owned = transfer.key_b64.clone();
                     let sender_owned = sender_id.to_string();
                     let filename_owned = filename.to_string();
                     let tx_ui_clone = tx_ui.clone();
                     let pseudo_clone = my_pseudo.to_string();
                     tokio::spawn(async move {
                         let media_dir = crate::crypto::get_media_dir(&pseudo_clone);
                         let safe_filename = filename_owned.replace('|', "_");
                         let mut path_buf = std::path::PathBuf::from(media_dir);
                         path_buf.push(safe_filename);
                         let save_path = path_buf.to_string_lossy().to_string();

                         if tokio::fs::write(&save_path, &owned_buffer).await.is_ok() {
                             let _ = tx_ui_clone.send(format!("FILE_RECV:{}:{}:{}:{}", sender_owned, filename_owned, key_owned, save_path));
                         }
                         owned_buffer.zeroize();
                     });
                 }
             }
        }
        else if sys_cmd == "ACK_CHUNK" && parts.len() >= 4 {
             let filename = std::str::from_utf8(parts[2]).unwrap_or("");
             let index: usize = std::str::from_utf8(parts[3]).unwrap_or("0").parse().unwrap_or(0);
             let transfer_key = format!("{}_{}", sender_id, filename);

             if let Some(transfer) = self.outgoing.get(&transfer_key) {
                 let next_index = index + 1;
                 if next_index < transfer.total_chunks {
                     let start = next_index * CHUNK_SIZE;
                     let end = std::cmp::min(start + CHUNK_SIZE, transfer.file_data.len());
                     
                     let mut payload = format!("SYS:FILE_CHUNK:{}:{}:", filename, next_index).into_bytes();
                     payload.extend_from_slice(&transfer.file_data[start..end]);

                     if let Some(dc) = data_channels.get(sender_id) {
                         let _ = dc.send(&bytes::Bytes::from(payload)).await;
                     }
                 } else {
                     self.outgoing.remove(&transfer_key); 
                 }
             }
        }
    }
}