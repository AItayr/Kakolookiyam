use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use webrtc::data_channel::RTCDataChannel;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use webrtc::api::APIBuilder;
use webrtc::api::media_engine::MediaEngine;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::ice_transport::ice_candidate::RTCIceCandidateInit;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use std::sync::Arc as StdArc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::chunking::TransferManager;
use super::webrtc_conn::create_peer_connection;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Signal {
    Register { id: String, pseudo: String, timestamp: u64, signature: String },
    Unregister { id: String },
    Offer { sdp: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    ChatOffer { sdp: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    Answer { sdp: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    Ice { candidate: String, sender_id: String, target_id: String, pseudo: String, timestamp: u64, signature: String },
    Ping { msg: String }
}


struct RateLimiter {
    hits: std::collections::HashMap<String, (u32, u64)>,
}

impl RateLimiter {
    fn new() -> Self { Self { hits: Default::default() } }
    fn check(&mut self, id: &str, max_hits: u32, window_secs: u64) -> bool {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        if self.hits.len() > 2000 {
            self.hits.retain(|_, (_, start)| now - *start < window_secs * 4);
        }
        let entry = self.hits.entry(id.to_string()).or_insert((0, now));
        if now - entry.1 >= window_secs { *entry = (1, now); return true; }
        if entry.0 >= max_hits { return false; }
        entry.0 += 1;
        true
    }
}

pub async fn start_p2p(
    mut rx_mic: tokio::sync::mpsc::Receiver<Vec<u8>>,
    tx_speaker: std::sync::mpsc::Sender<(usize, Vec<i16>)>,
    mut rx_ui: UnboundedReceiver<String>,
    tx_ui: UnboundedSender<String>,
    mut my_local_id: String,
    mut my_pseudo: String,
    my_secret_arr: [u8; 32],
    mut rx_secrets: tokio::sync::mpsc::UnboundedReceiver<[u8; 32]>,
) -> Result<(), Box<dyn std::error::Error>> {

    let mut my_secret: Vec<u8> = my_secret_arr.to_vec();
    let mut m = MediaEngine::default();
    m.register_default_codecs()?;
    let api = APIBuilder::new().with_media_engine(m).build();

    let audio_track = StdArc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: "audio/opus".to_owned(),
            clock_rate: 48000,
            channels: 2,
            ..Default::default()
        },
        "audio_p2p".to_owned(),
        "kakolookiyam_voice".to_owned(),
    ));

    let is_muted = Arc::new(AtomicBool::new(false));
    let audio_track_clone = Arc::clone(&audio_track);
    let is_muted_clone = Arc::clone(&is_muted);

    tokio::spawn(async move {
        while let Some(opus_packet) = rx_mic.recv().await {
            if !is_muted_clone.load(Ordering::Relaxed) {
                let sample_data = webrtc::media::Sample {
                    data: bytes::Bytes::from(opus_packet),
                    duration: std::time::Duration::from_millis(20),
                    ..Default::default()
                };
                let _ = audio_track_clone.write_sample(&sample_data).await;
            }
        }
    });

    let url = "wss://signal.kakolookiyam.ch"; // [MITIGATION] Route chiffrée par Reverse-Proxy (Suisse)
    let (ws_stream, _) = match connect_async(url).await {
        Ok(stream) => stream,
        Err(_) => {
            let _ = tx_ui.send("ERROR:SERVER_OFFLINE".to_string());
            return Ok(());
        }
    };
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let tstamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let sig = crate::crypto::sign_announcement(&my_secret_arr, &my_pseudo, tstamp);
    let reg = Signal::Register { id: my_local_id.clone(), pseudo: my_pseudo.clone(), timestamp: tstamp, signature: sig };
    let _ = ws_sender.send(Message::Text(serde_json::to_string(&reg).unwrap().into())).await;

    let (tx_signal, mut rx_signal) = tokio::sync::mpsc::channel::<Signal>(32);
    let mut peers: HashMap<String, (Arc<RTCPeerConnection>, std::sync::Arc<std::sync::atomic::AtomicBool>)> = HashMap::new();
    let mut pending_outbound_offers: HashMap<String, std::time::Instant> = HashMap::new();
    let mut pending_ice: HashMap<String, Vec<String>> = HashMap::new();
    let mut turn_user = String::new();
    let mut turn_pass = String::new();
    let (tx_dc, mut rx_dc) = tokio::sync::mpsc::unbounded_channel::<(String, Arc<RTCDataChannel>)>();
    let mut data_channels: HashMap<String, Arc<RTCDataChannel>> = HashMap::new();

    let (tx_chunks, mut rx_chunks) = tokio::sync::mpsc::unbounded_channel::<(String, Vec<u8>)>();
    let mut transfer_manager = TransferManager::new();
    let mut trusted_contacts: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut blocked_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut pending_chat: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut offer_limiter = RateLimiter::new();
    let mut global_limiter = RateLimiter::new();

    loop {
        tokio::select! {
            Some(chunk_data) = rx_chunks.recv() => {
                let sender_id: String = chunk_data.0;
                let raw_payload: Vec<u8> = chunk_data.1;
                transfer_manager.handle_message(&sender_id, &raw_payload, &data_channels, &tx_ui, &my_pseudo).await;
            }

            Some((tgt, dc)) = rx_dc.recv() => {
                data_channels.insert(tgt.clone(), std::sync::Arc::clone(&dc));
                if let Some(mut msgs) = pending_chat.remove(&tgt) {
                    for msg in msgs {
                        let _ = dc.send_text(msg).await;
                    }
                }
            }
            Some(cmd) = rx_ui.recv() => {
                if cmd.starts_with("CONTACTS_SYNC") {
                    trusted_contacts.clear();
                    let parts: Vec<&str> = cmd.split(':').collect();
                    for i in 1..parts.len() {
                        trusted_contacts.insert(parts[i].to_string());
                    }
                }
                else if cmd.starts_with("BLOCKED_SYNC") {
                    blocked_ids.clear();
                    let parts: Vec<&str> = cmd.split(':').collect();
                    for i in 1..parts.len() {
                        blocked_ids.insert(parts[i].to_string());
                    }
                }
                else if cmd.starts_with("REGISTER:") {
                    let parts: Vec<&str> = cmd.splitn(4, ':').collect();
                    if parts.len() == 4 || parts.len() == 3 {
                        my_local_id = parts[1].to_string();
                        my_pseudo = parts[2].to_string();
                        if let Ok(new_secret) = rx_secrets.try_recv() {
                            my_secret = new_secret.to_vec();
                        }
                        let _ = tx_signal.send(Signal::Register { id: my_local_id.clone(), pseudo: String::new(), timestamp: 0, signature: String::new() }).await;
                    } else if parts.len() == 2 {
                        my_local_id = parts[1].to_string();
                        let _ = tx_signal.send(Signal::Register { id: my_local_id.clone(), pseudo: String::new(), timestamp: 0, signature: String::new() }).await;
                    }
                }
                else if cmd.starts_with("LOGOUT:") {
                    let id = cmd.trim_start_matches("LOGOUT:").to_string();
                    let _ = tx_signal.send(Signal::Unregister { id }).await;

                    for (_, (pc, flag)) in peers.drain() {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = pc.close().await;
                    }
                    data_channels.clear();
                    transfer_manager.outgoing.clear();
                    transfer_manager.incoming.clear();
                }
                else if cmd.starts_with("CALL:") {
                    let full_target = cmd.trim_start_matches("CALL:").to_string();
                    let mut target_id = full_target.clone();
                    let mut grp_ctx = String::new();
                    
                    if let Some(idx) = full_target.find("|GRP:") {
                        grp_ctx = full_target[idx + 5..].to_string();
                        target_id = full_target[..idx].to_string();
                    }
                    if target_id.trim().is_empty() { continue; }
                    
                    let tx_ui_loading = tx_ui.clone();
                    let timeout_target_id = target_id.clone();
                    tokio::spawn(async move {
                        let _ = tx_ui_loading.send("LOADING:Allocation du relais Oracle (TURN UDP)...".to_string());
                        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        let _ = tx_ui_loading.send("LOADING:Génération des clés de session...".to_string());
                        tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
                        let _ = tx_ui_loading.send("LOADING:Handshake Cryptographique (DTLS)...".to_string());
                        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                        let _ = tx_ui_loading.send("LOADING:Canal P2P Zéro-Trace sécurisé...".to_string());
                        
                        tokio::time::sleep(tokio::time::Duration::from_secs(25)).await;
                        let _ = tx_ui_loading.send(format!("TIMEOUT:{}", timeout_target_id));
                    });

                    if let Some((old_pc, flag)) = peers.remove(&target_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = old_pc.close().await;
                    }
                    data_channels.remove(&target_id);
                    pending_ice.remove(&target_id);
                    transfer_manager.cleanup(&target_id);
                    let (pc, flag) = match create_peer_connection(
                        &api,
                        target_id.clone(),
                        my_local_id.clone(),
                        my_pseudo.clone(),
                        Some(Arc::clone(&audio_track)),
                        tx_speaker.clone(),
                        tx_signal.clone(),
                        tx_ui.clone(),
                        tx_dc.clone(),
                        tx_chunks.clone(),
                        true,
                        turn_user.clone(),
                        turn_pass.clone()
                    ).await {
                        Ok((p, f)) => (p, f),
                        Err(e) => {
                            let _ = tx_ui.send(format!("CHAT_RECV:err:Failed PC: {}", e));
                            continue;
                        }
                    };

                    let data_channel = match pc.create_data_channel("secure_text", Some(webrtc::data_channel::data_channel_init::RTCDataChannelInit { ordered: Some(true), max_retransmits: None, ..Default::default() })).await {
                        Ok(dc) => dc,
                        Err(_) => continue,
                    };
                    data_channels.insert(target_id.clone(), Arc::clone(&data_channel));

                    let d_open = Arc::clone(&data_channel);
                    let p = my_pseudo.clone();
                    let tgt_id_open = target_id.clone();
                    let tx_ui_open = tx_ui.clone();

                    data_channel.on_open(Box::new(move || {
                        let p2 = p.clone();
                        let tgt = tgt_id_open.clone();
                        let tx = tx_ui_open.clone();
                        Box::pin(async move {
                            let _ = tx.send(format!("CALL_ACTIVE:{}", tgt));
                            let _ = d_open.send_text(format!("{{\"type\":\"pseudo\",\"value\":\"{}\"}}", p2)).await;
                        })
                    }));

                    let tgt_id_msg = target_id.clone();
                    let tx_ui_msg = tx_ui.clone();
                    let tx_chunks_inner = tx_chunks.clone();
                    data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
                        let raw = msg.data.to_vec();
                        if raw.starts_with(b"SYS:FILE_") || raw.starts_with(b"SYS:ACK_") {
                            let _ = tx_chunks_inner.send((tgt_id_msg.clone(), raw));
                            return Box::pin(async move {});
                        }
                        if let Ok(text) = String::from_utf8(raw) {
                            if text.starts_with("{\"type\":\"pseudo\"") {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(friend_pseudo) = json["value"].as_str() {
                                        let _ = tx_ui_msg.send(format!("CONTACT:{}:{}", tgt_id_msg, friend_pseudo));
                                        return Box::pin(async move {});
                                    }
                                }
                            } else {
                                let _ = tx_ui_msg.send(format!("CHAT_RECV:{}:{}", tgt_id_msg, text));
                            }
                        }
                        Box::pin(async move {})
                    }));

                    if let Ok(offer) = pc.create_offer(None).await {
                        let pc_clone = Arc::clone(&pc);
                        let tx_sig = tx_signal.clone();
                        let my_id = my_local_id.clone();
                        let tgt_id = target_id.clone();
                        let grp_context_clone = grp_ctx.clone();
                        let my_secret_clone = my_secret.clone();
                        let my_pseudo_clone = my_pseudo.clone();
                        tokio::spawn(async move {
                            if pc_clone.set_local_description(offer.clone()).await.is_ok() {
                                let mut final_sdp = offer.sdp;
                                if !grp_context_clone.is_empty() {
                                    final_sdp = format!("{}|||GRP:{}", final_sdp, grp_context_clone);
                                }
                                let tstamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                let sig = crate::crypto::sign_announcement(&my_secret_clone, &my_pseudo_clone, tstamp);
                                let _ = tx_sig.send(Signal::Offer { sdp: final_sdp, sender_id: my_id, target_id: tgt_id, pseudo: my_pseudo_clone.clone(), timestamp: tstamp, signature: sig }).await;
                            }
                        });
                        peers.insert(target_id.clone(), (pc, flag));
                        pending_outbound_offers.insert(target_id.clone(), std::time::Instant::now());
                    }
                }
                else if cmd.starts_with("ACCEPT:") {
                    let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let sender_id = parts[1].to_string();
                        let sdp = parts[2].to_string();

                        if let Some((old_pc, flag)) = peers.remove(&sender_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                            let _ = old_pc.close().await;
                        }
                        data_channels.remove(&sender_id);
                                        if let Ok((pc, flag)) = create_peer_connection(
                            &api,
                            sender_id.clone(),
                            my_local_id.clone(),
                            my_pseudo.clone(),
                            Some(Arc::clone(&audio_track)),
                            tx_speaker.clone(),
                            tx_signal.clone(),
                            tx_ui.clone(),
                            tx_dc.clone(),
                            tx_chunks.clone(),
                            true,
                            turn_user.clone(),
                            turn_pass.clone()
                        ).await {
                            peers.insert(sender_id.clone(), (Arc::clone(&pc), std::sync::Arc::clone(&flag)));

                            let mut desc = RTCSessionDescription::default();
                            desc.sdp_type = RTCSdpType::Offer; desc.sdp = sdp;
                            if pc.set_remote_description(desc).await.is_ok() {
                                if let Some(candidates) = pending_ice.remove(&sender_id) {
                                    for candidate in candidates {
                                        let ice_init = RTCIceCandidateInit { candidate: candidate.clone(), ..Default::default() };
                                        let _ = pc.add_ice_candidate(ice_init).await;
                                    }
                                }
                                if let Ok(answer) = pc.create_answer(None).await {
                                    let pc_clone = Arc::clone(&pc);
                                    let tx_sig = tx_signal.clone();
                                    let my_id = my_local_id.clone();
                                    let tgt_id = sender_id.clone();
                                    tokio::spawn(async move {
                                        let mut gather_complete = pc_clone.gathering_complete_promise().await;
                                        if pc_clone.set_local_description(answer).await.is_ok() {
                                            let _ = gather_complete.recv().await;
                                            if let Some(local_desc) = pc_clone.local_description().await {
                                                let _ = tx_sig.send(Signal::Answer { sdp: local_desc.sdp, sender_id: my_id, target_id: tgt_id, pseudo: String::new(), timestamp: 0, signature: String::new() }).await;
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
                else if cmd.starts_with("REJECT:") {
                    let sender_id = cmd.trim_start_matches("REJECT:").to_string();
                    pending_ice.remove(&sender_id);
                    if let Some((pc, flag)) = peers.remove(&sender_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = pc.close().await;
                    }
                    transfer_manager.cleanup(&sender_id);
                    
                    // [MED-4 UX] Signale immédiatement à l'appelant que la ligne est occupée !
                    let _ = tx_signal.send(Signal::Answer { sdp: "BUSY".to_string(), sender_id: my_local_id.clone(), target_id: sender_id, pseudo: String::new(), timestamp: 0, signature: String::new() }).await;
                }
                else if cmd.starts_with("HANGUP:") {
                    let target_id = cmd.trim_start_matches("HANGUP:").to_string();
                    if let Some((pc, flag)) = peers.remove(&target_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                        let _ = pc.close().await;
                    }
                    data_channels.remove(&target_id);
                    pending_ice.remove(&target_id);
                    transfer_manager.cleanup(&target_id);
                }
                else if cmd == "MUTE:on" {
                    is_muted.store(true, Ordering::Relaxed);
                }
                else if cmd == "MUTE:off" {
                    is_muted.store(false, Ordering::Relaxed);
                }
                else if cmd.starts_with("CHAT_SEND_IF_OPEN:") {
                    let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let target_id = parts[1].to_string();
                        let text = parts[2].to_string();
                        if let Some(dc) = data_channels.get(&target_id) {
                            let _ = dc.send_text(text).await;
                        }
                    }
                }
                else if cmd.starts_with("CHAT_SEND:") {
                    let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let target_id = parts[1].to_string();
                        let text = parts[2].to_string();

                        if let Some(dc) = data_channels.get(&target_id) {
                            let _ = dc.send_text(text).await;
                        } else {
                            if let Some((old_pc, flag)) = peers.remove(&target_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                                let _ = old_pc.close().await;
                            }

                            if let Ok((pc, flag)) = create_peer_connection(
                                &api,
                                target_id.clone(),
                                my_local_id.clone(),
                                my_pseudo.clone(),
                                None,
                                tx_speaker.clone(),
                                tx_signal.clone(),
                                tx_ui.clone(),
                                tx_dc.clone(),
                                tx_chunks.clone(),
                                false,
                                turn_user.clone(),
                                turn_pass.clone()
                            ).await {
                                if let Ok(data_channel) = pc.create_data_channel("secure_text", Some(webrtc::data_channel::data_channel_init::RTCDataChannelInit { ordered: Some(true), max_retransmits: None, ..Default::default() })).await {
                                    data_channels.insert(target_id.clone(), Arc::clone(&data_channel));

                                    let d_open = Arc::clone(&data_channel);
                                    let p = my_pseudo.clone();
                                    let txt_to_send = text.clone();

                                    data_channel.on_open(Box::new(move || {
                                        let p2 = p.clone();
                                        let txt = txt_to_send.clone();
                                        let d = Arc::clone(&d_open);
                                        Box::pin(async move {
                                            let _ = d.send_text(format!("{{\"type\":\"pseudo\",\"value\":\"{}\"}}", p2)).await;
                                            let _ = d.send_text(txt).await;
                                        })
                                    }));

                                    let tgt_id_msg = target_id.clone();
                                    let tx_ui_msg = tx_ui.clone();
                                    let tx_chunks_inner = tx_chunks.clone();
                                    data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
                        let raw = msg.data.to_vec();
                        if raw.starts_with(b"SYS:FILE_") || raw.starts_with(b"SYS:ACK_") {
                            let _ = tx_chunks_inner.send((tgt_id_msg.clone(), raw));
                            return Box::pin(async move {});
                        }
                        if let Ok(text) = String::from_utf8(raw) {
                            if text.starts_with("{\"type\":\"pseudo\"") {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(friend_pseudo) = json["value"].as_str() {
                                        let _ = tx_ui_msg.send(format!("CONTACT:{}:{}", tgt_id_msg, friend_pseudo));
                                        return Box::pin(async move {});
                                    }
                                }
                            } else {
                                let _ = tx_ui_msg.send(format!("CHAT_RECV:{}:{}", tgt_id_msg, text));
                            }
                        }
                        Box::pin(async move {})
                    }));

                                    if let Ok(offer) = pc.create_offer(None).await {
                        let pc_clone = Arc::clone(&pc); let tx_sig = tx_signal.clone(); let my_id = my_local_id.clone(); let tgt_id = target_id.clone();
                        let my_secret_clone = my_secret.clone();
                        let my_pseudo_clone = my_pseudo.clone();
                        tokio::spawn(async move {
                            if pc_clone.set_local_description(offer.clone()).await.is_ok() {
                                let tstamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                let sig = crate::crypto::sign_announcement(&my_secret_clone, &my_pseudo_clone, tstamp);
                                let _ = tx_sig.send(Signal::ChatOffer { sdp: offer.sdp, sender_id: my_id, target_id: tgt_id, pseudo: my_pseudo_clone.clone(), timestamp: tstamp, signature: sig }).await;
                            }
                        });
                        peers.insert(target_id.clone(), (pc, flag));
                        pending_outbound_offers.insert(target_id.clone(), std::time::Instant::now());
                    }
                                }
                            }
                        }
                    }
                }
                else if cmd.starts_with("FILE_SEND_INIT:") {
                    let parts: Vec<&str> = cmd.splitn(5, ':').collect();
                    if parts.len() == 5 {
                        let target_id = parts[1].to_string();
                        let filename = parts[2].to_string();
                        let key_b64 = parts[3].to_string();
                        let enc_path = parts[4].to_string();

                        if let Ok(file_data) = tokio::fs::read(&enc_path).await {
                            let total_chunks = (file_data.len() + super::chunking::CHUNK_SIZE - 1) / super::chunking::CHUNK_SIZE;

                            let transfer_key = format!("{}_{}", target_id, filename);
                            transfer_manager.outgoing.insert(transfer_key, super::chunking::OutgoingTransfer {
                                file_data,
                                total_chunks,
                            });

                            let meta_msg = format!("SYS:FILE_META:{}:{}:{}", filename, total_chunks, key_b64);

                            if let Some(dc) = data_channels.get(&target_id) {
                                let _ = dc.send_text(meta_msg).await;
                            } else {
                                if peers.contains_key(&target_id) { continue; }

                                if let Ok((pc, flag)) = create_peer_connection(
                                    &api,
                                    target_id.clone(),
                                    my_local_id.clone(),
                                    my_pseudo.clone(),
                                    None,
                                    tx_speaker.clone(),
                                    tx_signal.clone(),
                                    tx_ui.clone(),
                                    tx_dc.clone(),
                                    tx_chunks.clone(),
                                    false,
                                    turn_user.clone(),
                                    turn_pass.clone()
                                ).await {
                                    if let Ok(data_channel) = pc.create_data_channel("secure_text", Some(webrtc::data_channel::data_channel_init::RTCDataChannelInit { ordered: Some(true), max_retransmits: None, ..Default::default() })).await {
                                        data_channels.insert(target_id.clone(), Arc::clone(&data_channel));

                                        let d_open = Arc::clone(&data_channel);
                                        let p = my_pseudo.clone();
                                        let m_msg = meta_msg.clone();

                                        data_channel.on_open(Box::new(move || {
                                            let p2 = p.clone();
                                            let m = m_msg.clone();
                                            let d = Arc::clone(&d_open);
                                            Box::pin(async move {
                                                let _ = d.send_text(format!("{{\"type\":\"pseudo\",\"value\":\"{}\"}}", p2)).await;
                                                let _ = d.send_text(m).await;
                                            })
                                        }));

                                        let tgt_id_msg = target_id.clone();
                                        let tx_ui_msg = tx_ui.clone();
                                        let tx_chunks_inner = tx_chunks.clone();
                                        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
                        let raw = msg.data.to_vec();
                        if raw.starts_with(b"SYS:FILE_") || raw.starts_with(b"SYS:ACK_") {
                            let _ = tx_chunks_inner.send((tgt_id_msg.clone(), raw));
                            return Box::pin(async move {});
                        }
                        if let Ok(text) = String::from_utf8(raw) {
                            if text.starts_with("{\"type\":\"pseudo\"") {
                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(friend_pseudo) = json["value"].as_str() {
                                        let _ = tx_ui_msg.send(format!("CONTACT:{}:{}", tgt_id_msg, friend_pseudo));
                                        return Box::pin(async move {});
                                    }
                                }
                            } else {
                                let _ = tx_ui_msg.send(format!("CHAT_RECV:{}:{}", tgt_id_msg, text));
                            }
                        }
                        Box::pin(async move {})
                    }));

                                        if let Ok(offer) = pc.create_offer(None).await {
                        let pc_clone = Arc::clone(&pc); let tx_sig = tx_signal.clone(); let my_id = my_local_id.clone(); let tgt_id = target_id.clone();
                        let my_secret_clone = my_secret.clone();
                        let my_pseudo_clone = my_pseudo.clone();
                        tokio::spawn(async move {
                            if pc_clone.set_local_description(offer.clone()).await.is_ok() {
                                let tstamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                let sig = crate::crypto::sign_announcement(&my_secret_clone, &my_pseudo_clone, tstamp);
                                let _ = tx_sig.send(Signal::ChatOffer { sdp: offer.sdp, sender_id: my_id, target_id: tgt_id, pseudo: my_pseudo_clone.clone(), timestamp: tstamp, signature: sig }).await;
                            }
                        });
                        peers.insert(target_id.clone(), (pc, flag));
                        pending_outbound_offers.insert(target_id.clone(), std::time::Instant::now());
                    }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(mut signal) = rx_signal.recv() => {
                let tstamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                let sig = crate::crypto::sign_announcement(&my_secret, &my_pseudo, tstamp);
                match &mut signal {
                    Signal::Answer { pseudo, timestamp, signature, .. } |
                    Signal::Ice { pseudo, timestamp, signature, .. } => {
                        *pseudo = my_pseudo.clone();
                        *timestamp = tstamp;
                        *signature = sig;
                    },
                    Signal::Register { id: _id, pseudo, timestamp, signature } => {
                        // Normally ID is already set
                        *pseudo = my_pseudo.clone();
                        *timestamp = tstamp;
                        *signature = crate::crypto::sign_announcement(&my_secret, &my_pseudo, tstamp);
                    },
                    _ => {}
                }
                
                if let Ok(json) = serde_json::to_string(&signal) {
                    let _ = ws_sender.send(Message::Text(json.into())).await;
                }
            }
            Some(result) = ws_receiver.next() => {
                match result {
                    Ok(response) => {
                        if let Ok(text) = response.into_text() {

                            if text == "ERROR:ALREADY_CONNECTED" || text == "SUCCESS:REGISTERED" {
                                let _ = tx_ui.send(text.to_string());
                                continue;
                            }
                            if text.starts_with("TURN_AUTH|") {
                                let parts: Vec<&str> = text.splitn(3, '|').collect();
                                if parts.len() == 3 {
                                    turn_user = parts[1].to_string();
                                    turn_pass = parts[2].to_string();
                                }
                                continue;
                            }

                            if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                                match signal {
                                    Signal::Offer { mut sdp, sender_id, pseudo, timestamp, signature, .. } => {
                                                                                if blocked_ids.contains(&sender_id) { continue; }
                                        let is_known = trusted_contacts.contains(&sender_id);
                                        let (max_hits, window) = if is_known { (10, 60) } else { (1, 30) };
                                        if !offer_limiter.check(&sender_id, max_hits, window) { continue; }
                                        if !is_known && !global_limiter.check("__global_unknown__", 2, 60) { continue; } // [MED-4] Limite stricte pour les inconnus

                                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                        if timestamp < now - 60 || timestamp > now + 60 { continue; }
                                        if !crate::crypto::verify_announcement(&sender_id, &pseudo, timestamp, &signature) { continue; }
                                        
                                          if let Some(_time) = pending_outbound_offers.get(&sender_id) {
                                                // [REMOVED GLARE REJECTION] 
                                                // Dropping incoming offers caused a 30s deadlock if the remote was offline when our initial offer was sent!
                                                pending_outbound_offers.remove(&sender_id);
                                            }
if let Some((old_pc, flag)) = peers.remove(&sender_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                                            let _ = old_pc.close().await;
                                        }
                                        data_channels.remove(&sender_id);
                                        pending_ice.remove(&sender_id);
                                        
                                        let mut grp_context = String::new();
                                        if let Some(idx) = sdp.find("|||GRP:") {
                                            grp_context = sdp[idx + 7..].to_string();
                                            sdp = sdp[..idx].to_string();
                                        }
                                        
                                        let _ = tx_ui.send(format!("INCOMING_CALL:{}:{}:{}", sender_id, grp_context, sdp));
                                    }
                                    Signal::ChatOffer { sdp, sender_id, pseudo, timestamp, signature, .. } => {
                                                                                if blocked_ids.contains(&sender_id) { continue; }
                                        let is_known = trusted_contacts.contains(&sender_id);
                                        let (max_hits, window) = if is_known { (10, 60) } else { (1, 30) };
                                        if !offer_limiter.check(&sender_id, max_hits, window) { continue; }
                                        if !is_known && !global_limiter.check("__global_unknown__", 2, 60) { continue; } // [MED-4] Limite stricte pour les inconnus

                                        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                        if timestamp < now - 60 || timestamp > now + 60 { continue; }
                                        if !crate::crypto::verify_announcement(&sender_id, &pseudo, timestamp, &signature) { continue; }
                                          if let Some(_time) = pending_outbound_offers.get(&sender_id) {
                                                // [REMOVED GLARE REJECTION] 
                                                // Dropping incoming offers caused a 30s deadlock if the remote was offline when our initial offer was sent!
                                                pending_outbound_offers.remove(&sender_id);
                                            }
if let Some((old_pc, flag)) = peers.remove(&sender_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                                            let _ = old_pc.close().await;
                                        }
                                        data_channels.remove(&sender_id);

                                        if let Ok((pc, flag)) = create_peer_connection(
                                            &api, sender_id.clone(), my_local_id.clone(), my_pseudo.clone(),
                                            None, tx_speaker.clone(), tx_signal.clone(), tx_ui.clone(), tx_dc.clone(), tx_chunks.clone(), false,
                                            turn_user.clone(), turn_pass.clone()
                                        ).await {
                                            peers.insert(sender_id.clone(), (Arc::clone(&pc), std::sync::Arc::clone(&flag)));
                                            let mut desc = RTCSessionDescription::default();
                                            desc.sdp_type = RTCSdpType::Offer; desc.sdp = sdp;
                                            if pc.set_remote_description(desc).await.is_ok() {
                                                if let Some(candidates) = pending_ice.remove(&sender_id) {
                                                    for candidate in candidates {
                                                        let ice_init = RTCIceCandidateInit { candidate: candidate.clone(), ..Default::default() };
                                                        let _ = pc.add_ice_candidate(ice_init).await;
                                                    }
                                                }
                                                if let Ok(answer) = pc.create_answer(None).await {
                                    if pc.set_local_description(answer.clone()).await.is_ok() {
                                        let _ = tx_signal.send(Signal::Answer { sdp: answer.sdp, sender_id: my_local_id.clone(), target_id: sender_id, pseudo: String::new(), timestamp: 0, signature: String::new() }).await;
                                    }
                                }
                                            }
                                        }
                                    }
                                    Signal::Answer { sdp, sender_id, pseudo, timestamp, signature, .. } => {
                                        pending_outbound_offers.remove(&sender_id);
                                        if !crate::crypto::verify_announcement(&sender_id, &pseudo, timestamp, &signature) { continue; }

                                        if sdp == "BUSY" {
                                            let _ = tx_ui.send(format!("CALL_BUSY:{}", sender_id));
                                            if let Some((pc, flag)) = peers.remove(&sender_id) {
 flag.store(false, std::sync::atomic::Ordering::Relaxed);
                                                let _ = pc.close().await;
                                            }
                                            data_channels.remove(&sender_id);
                                            continue;
                                        }

                                        if let Some((pc, _flag)) = peers.get(&sender_id) {
                                            let mut desc = RTCSessionDescription::default();
                                            desc.sdp_type = RTCSdpType::Answer; desc.sdp = sdp;
                                            if pc.set_remote_description(desc).await.is_ok() {
                                                if let Some(candidates) = pending_ice.remove(&sender_id) {
                                                    for candidate in candidates {
                                                        let ice_init = RTCIceCandidateInit { candidate: candidate.clone(), ..Default::default() };
                                                        let _ = pc.add_ice_candidate(ice_init).await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    
                                    Signal::Ice { candidate, sender_id, pseudo, timestamp, signature, .. } => {
                                        if !crate::crypto::verify_announcement(&sender_id, &pseudo, timestamp, &signature) { continue; }

                                        let mut handled = false;
                                        if let Some((pc, _flag)) = peers.get(&sender_id) {
                                            if pc.remote_description().await.is_some() {
                                                let ice_init = RTCIceCandidateInit { candidate: candidate.clone(), ..Default::default() };
                                                let _ = pc.add_ice_candidate(ice_init).await;
                                                handled = true;
                                            }
                                        }
                                        if !handled {
                                            pending_ice.entry(sender_id).or_default().push(candidate);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    },
                    Err(_) => break,
                }
            }
        }
    }
    Ok(())
}




















