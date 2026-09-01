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
    Register { id: String },
    Unregister { id: String },
    Offer { sdp: String, sender_id: String, target_id: String },
    ChatOffer { sdp: String, sender_id: String, target_id: String },
    Answer { sdp: String, sender_id: String, target_id: String },
    Ice { candidate: String, sender_id: String, target_id: String },
    Ping { msg: String }
}

pub async fn start_p2p(
    mut rx_mic: tokio::sync::mpsc::Receiver<Vec<u8>>,
    tx_speaker: std::sync::mpsc::Sender<Vec<i16>>,
    mut rx_ui: UnboundedReceiver<String>,
    tx_ui: UnboundedSender<String>,
    mut my_local_id: String,
    mut my_pseudo: String,
) -> Result<(), Box<dyn std::error::Error>> {

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

    let url = "ws://89.168.62.93:8080";
    let (ws_stream, _) = match connect_async(url).await {
        Ok(stream) => stream,
        Err(_) => {
            let _ = tx_ui.send("ERROR:SERVER_OFFLINE".to_string());
            return Ok(());
        }
    };
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    let reg = Signal::Register { id: my_local_id.clone() };
    let _ = ws_sender.send(Message::Text(serde_json::to_string(&reg).unwrap().into())).await;

    let (tx_signal, mut rx_signal) = tokio::sync::mpsc::channel::<Signal>(32);
    let mut peers: HashMap<String, Arc<RTCPeerConnection>> = HashMap::new();
    let (tx_dc, mut rx_dc) = tokio::sync::mpsc::unbounded_channel::<(String, Arc<RTCDataChannel>)>();
    let mut data_channels: HashMap<String, Arc<RTCDataChannel>> = HashMap::new();

    let (tx_chunks, mut rx_chunks) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();
    let mut transfer_manager = TransferManager::new();

    loop {
        tokio::select! {
            Some(chunk_data) = rx_chunks.recv() => {
                let sender_id: String = chunk_data.0;
                let text: String = chunk_data.1;
                transfer_manager.handle_message(&sender_id, &text, &data_channels, &tx_ui, &my_pseudo).await;
            }

            Some((tgt, dc)) = rx_dc.recv() => {
                data_channels.insert(tgt, dc);
            }
            Some(cmd) = rx_ui.recv() => {
                if cmd.starts_with("REGISTER:") {
                    let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        my_local_id = parts[1].to_string();
                        my_pseudo = parts[2].to_string();
                        let _ = tx_signal.send(Signal::Register { id: my_local_id.clone() }).await;
                    } else if parts.len() == 2 {
                        my_local_id = parts[1].to_string();
                        let _ = tx_signal.send(Signal::Register { id: my_local_id.clone() }).await;
                    }
                }
                else if cmd.starts_with("LOGOUT:") {
                    let id = cmd.trim_start_matches("LOGOUT:").to_string();
                    let _ = tx_signal.send(Signal::Unregister { id }).await;

                    for (_, pc) in peers.drain() {
                        let _ = pc.close().await;
                    }
                    data_channels.clear();
                    transfer_manager.outgoing.clear();
                    transfer_manager.incoming.clear();
                }
                else if cmd.starts_with("CALL:") {
                    let target_id = cmd.trim_start_matches("CALL:").to_string();
                    if target_id.trim().is_empty() { continue; }

                    if let Some(old_pc) = peers.remove(&target_id) {
                        let _ = old_pc.close().await;
                    }
                    data_channels.remove(&target_id);

                    let pc = match create_peer_connection(
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
                        true
                    ).await {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    let data_channel = match pc.create_data_channel("secure_text", None).await {
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
                        let text = String::from_utf8_lossy(&msg.data);
                        if text.starts_with("SYS:FILE_META:") || text.starts_with("SYS:ACK_META:") || text.starts_with("SYS:FILE_CHUNK:") || text.starts_with("SYS:ACK_CHUNK:") {
                            let _ = tx_chunks_inner.send((tgt_id_msg.clone(), text.into_owned()));
                        }
                        else if text.starts_with("{\"type\":\"pseudo\"") {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                if let Some(friend_pseudo) = json["value"].as_str() {
                                    let _ = tx_ui_msg.send(format!("CONTACT:{}:{}", tgt_id_msg, friend_pseudo));
                                    return Box::pin(async move {});
                                }
                            }
                        }
                        else {
                            let _ = tx_ui_msg.send(format!("CHAT_RECV:{}:{}", tgt_id_msg, text));
                        }
                        Box::pin(async move {})
                    }));

                    if let Ok(offer) = pc.create_offer(None).await {
                        if pc.set_local_description(offer.clone()).await.is_ok() {
                            let _ = tx_signal.send(Signal::Offer { sdp: offer.sdp, sender_id: my_local_id.clone(), target_id: target_id.clone() }).await;
                            peers.insert(target_id, pc);
                        }
                    }
                }
                else if cmd.starts_with("ACCEPT:") {
                    let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let sender_id = parts[1].to_string();
                        let sdp = parts[2].to_string();

                        if let Some(old_pc) = peers.remove(&sender_id) {
                            let _ = old_pc.close().await;
                        }
                        data_channels.remove(&sender_id);

                        if let Ok(pc) = create_peer_connection(
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
                            true
                        ).await {
                            peers.insert(sender_id.clone(), Arc::clone(&pc));

                            let mut desc = RTCSessionDescription::default();
                            desc.sdp_type = RTCSdpType::Offer; desc.sdp = sdp;
                            if pc.set_remote_description(desc).await.is_ok() {
                                if let Ok(answer) = pc.create_answer(None).await {
                                    if pc.set_local_description(answer.clone()).await.is_ok() {
                                        let _ = tx_signal.send(Signal::Answer { sdp: answer.sdp, sender_id: my_local_id.clone(), target_id: sender_id }).await;
                                    }
                                }
                            }
                        }
                    }
                }
                else if cmd.starts_with("REJECT:") {
                    let sender_id = cmd.trim_start_matches("REJECT:").to_string();
                    if let Some(pc) = peers.remove(&sender_id) {
                        let _ = pc.close().await;
                    }
                }
                else if cmd.starts_with("HANGUP:") {
                    let target_id = cmd.trim_start_matches("HANGUP:").to_string();
                    if let Some(pc) = peers.remove(&target_id) {
                        let _ = pc.close().await;
                    }
                    data_channels.remove(&target_id);
                }
                else if cmd == "MUTE:on" {
                    is_muted.store(true, Ordering::Relaxed);
                }
                else if cmd == "MUTE:off" {
                    is_muted.store(false, Ordering::Relaxed);
                }
                else if cmd.starts_with("CHAT_SEND:") {
                    let parts: Vec<&str> = cmd.splitn(3, ':').collect();
                    if parts.len() == 3 {
                        let target_id = parts[1].to_string();
                        let text = parts[2].to_string();

                        if let Some(dc) = data_channels.get(&target_id) {
                            let _ = dc.send_text(text).await;
                        } else {
                            if let Some(old_pc) = peers.remove(&target_id) {
                                let _ = old_pc.close().await;
                            }

                            if let Ok(pc) = create_peer_connection(
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
                                false
                            ).await {
                                if let Ok(data_channel) = pc.create_data_channel("secure_text", None).await {
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
                                        let text = String::from_utf8_lossy(&msg.data);
                                        if text.starts_with("SYS:FILE_META:") || text.starts_with("SYS:ACK_META:") || text.starts_with("SYS:FILE_CHUNK:") || text.starts_with("SYS:ACK_CHUNK:") {
                                            let _ = tx_chunks_inner.send((tgt_id_msg.clone(), text.into_owned()));
                                        }
                                        else if text.starts_with("{\"type\":\"pseudo\"") {
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                                if let Some(friend_pseudo) = json["value"].as_str() {
                                                    let _ = tx_ui_msg.send(format!("CONTACT:{}:{}", tgt_id_msg, friend_pseudo));
                                                    return Box::pin(async move {});
                                                }
                                            }
                                        }
                                        else {
                                            let _ = tx_ui_msg.send(format!("CHAT_RECV:{}:{}", tgt_id_msg, text));
                                        }
                                        Box::pin(async move {})
                                    }));

                                    if let Ok(offer) = pc.create_offer(None).await {
                                        if pc.set_local_description(offer.clone()).await.is_ok() {
                                            let _ = tx_signal.send(Signal::ChatOffer { sdp: offer.sdp, sender_id: my_local_id.clone(), target_id: target_id.clone() }).await;
                                            peers.insert(target_id, pc);
                                        }
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

                                if let Ok(pc) = create_peer_connection(
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
                                    false
                                ).await {
                                    if let Ok(data_channel) = pc.create_data_channel("secure_text", None).await {
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
                                            let text = String::from_utf8_lossy(&msg.data);
                                            if text.starts_with("SYS:FILE_META:") || text.starts_with("SYS:ACK_META:") || text.starts_with("SYS:FILE_CHUNK:") || text.starts_with("SYS:ACK_CHUNK:") {
                                                let _ = tx_chunks_inner.send((tgt_id_msg.clone(), text.into_owned()));
                                            }
                                            else if text.starts_with("{\"type\":\"pseudo\"") {
                                                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                                                    if let Some(friend_pseudo) = json["value"].as_str() {
                                                        let _ = tx_ui_msg.send(format!("CONTACT:{}:{}", tgt_id_msg, friend_pseudo));
                                                        return Box::pin(async move {});
                                                    }
                                                }
                                            }
                                            else {
                                                let _ = tx_ui_msg.send(format!("CHAT_RECV:{}:{}", tgt_id_msg, text));
                                            }
                                            Box::pin(async move {})
                                        }));

                                        if let Ok(offer) = pc.create_offer(None).await {
                                            if pc.set_local_description(offer.clone()).await.is_ok() {
                                                let _ = tx_signal.send(Signal::ChatOffer { sdp: offer.sdp, sender_id: my_local_id.clone(), target_id: target_id.clone() }).await;
                                                peers.insert(target_id, pc);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(signal) = rx_signal.recv() => {
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

                            if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                                match signal {
                                    Signal::Offer { sdp, sender_id, .. } => {
                                        let _ = tx_ui.send(format!("INCOMING_CALL:{}:{}", sender_id, sdp));
                                    }
                                    Signal::ChatOffer { sdp, sender_id, .. } => {
                                        if let Some(old_pc) = peers.remove(&sender_id) {
                                            let _ = old_pc.close().await;
                                        }
                                        data_channels.remove(&sender_id);

                                        if let Ok(pc) = create_peer_connection(
                                            &api,
                                            sender_id.clone(),
                                            my_local_id.clone(),
                                            my_pseudo.clone(),
                                            None,
                                            tx_speaker.clone(),
                                            tx_signal.clone(),
                                            tx_ui.clone(),
                                            tx_dc.clone(),
                                            tx_chunks.clone(),
                                            false
                                        ).await {
                                            peers.insert(sender_id.clone(), Arc::clone(&pc));

                                            let mut desc = RTCSessionDescription::default();
                                            desc.sdp_type = RTCSdpType::Offer; desc.sdp = sdp;
                                            if pc.set_remote_description(desc).await.is_ok() {
                                                if let Ok(answer) = pc.create_answer(None).await {
                                                    if pc.set_local_description(answer.clone()).await.is_ok() {
                                                        let _ = tx_signal.send(Signal::Answer { sdp: answer.sdp, sender_id: my_local_id.clone(), target_id: sender_id }).await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Signal::Answer { sdp, sender_id, .. } => {
                                        if let Some(pc) = peers.get(&sender_id) {
                                            let mut desc = RTCSessionDescription::default();
                                            desc.sdp_type = RTCSdpType::Answer; desc.sdp = sdp;
                                            let _ = pc.set_remote_description(desc).await;
                                        }
                                    }
                                    Signal::Ice { candidate, sender_id, .. } => {
                                        if let Some(pc) = peers.get(&sender_id) {
                                            if pc.remote_description().await.is_some() {
                                                let ice_init = RTCIceCandidateInit { candidate, ..Default::default() };
                                                let _ = pc.add_ice_candidate(ice_init).await;
                                            }
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