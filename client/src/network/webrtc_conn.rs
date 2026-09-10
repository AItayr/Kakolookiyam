use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use std::sync::Arc as StdArc;

static TRACK_ID_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
use webrtc::api::API;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::policy::ice_transport_policy::RTCIceTransportPolicy;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;

use super::router::Signal;

pub async fn create_peer_connection(
    api: &API,
    target_id: String,
    my_id: String,
    my_pseudo: String,
    audio_track: Option<StdArc<TrackLocalStaticSample>>,
    tx_speaker: std::sync::mpsc::Sender<(usize, Vec<i16>)>,
    tx_signal: tokio::sync::mpsc::Sender<Signal>,
    tx_ui: UnboundedSender<String>,
    tx_dc: UnboundedSender<(String, Arc<RTCDataChannel>)>,
    tx_chunks: UnboundedSender<(String, String)>,
    is_call: bool,
) -> Result<Arc<RTCPeerConnection>, Box<dyn std::error::Error>> {
    
    // 1. STRICT RELAY ZERO-TRACE CONFIGURATION
    // Core engine will literally refuse to touch your local NAT, bypassing IP leaks entirely!
    let config = RTCConfiguration {
        ice_transport_policy: RTCIceTransportPolicy::Relay,
        ice_servers: vec![
            RTCIceServer {
                urls: vec!["turn:89.168.62.93:3478".to_owned()],
                username: "kako_relais".to_owned(),
                credential: "cX@XctAfrSym5ak8".to_owned(),
                ..Default::default()
            }
        ],
        ..Default::default()
    };
    
    let pc = Arc::new(api.new_peer_connection(config).await?);

    let tx_ui_state = tx_ui.clone();
    let tgt_state = target_id.clone();

    pc.on_peer_connection_state_change(Box::new(move |state| {
        let tx = tx_ui_state.clone();
        let tgt = tgt_state.clone();
        Box::pin(async move {
            if state == RTCPeerConnectionState::Failed || state == RTCPeerConnectionState::Disconnected {
                let _ = tx.send(format!("CALL_ENDED:{}", tgt));
            }
        })
    }));

    // RESTORE TRICKLE ICE FOR INSANE FLUIDITY
    let tx_sig_ice = tx_signal.clone();
    let target_ice = target_id.clone();
    let my_id_ice = my_id.clone();
    
    pc.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
        let tx_sig_ice = tx_sig_ice.clone();
        let target = target_ice.clone();
        let sender = my_id_ice.clone();
        
        Box::pin(async move {
            if let Some(candidate) = c {
                if let Ok(json) = candidate.to_json() {
                    let _ = tx_sig_ice.send(Signal::Ice {
                        candidate: json.candidate,
                        sender_id: sender,
                        target_id: target
                    }).await;
                }
            }
        })
    }));

    if let Some(track) = audio_track {
        let tx_spk = tx_speaker.clone();

        pc.on_track(Box::new(move |track, _, _| {
            let tx_spk = tx_spk.clone();
            let track_id = TRACK_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async move {
                tokio::spawn(async move {
                    let mut decoder = audiopus::coder::Decoder::new(
                        audiopus::SampleRate::Hz48000,
                        audiopus::Channels::Stereo
                    ).unwrap();

                    let track = track;
                    while let Ok((rtp_packet, _)) = track.read_rtp().await {
                        let mut decoded_pcm = vec![0i16; 1920 * 2];
                        if let Ok(len) = decoder.decode(Some(rtp_packet.payload.as_ref()), &mut decoded_pcm, false) {
                            decoded_pcm.truncate(len * 2);
                            let _ = tx_spk.send((track_id, decoded_pcm));
                        }
                    }
                });
            })
        }));

        let rtp_sender = pc.add_track(
            StdArc::clone(&track) as StdArc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>
        ).await?;

        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        });
    }

    let target_id_msg = target_id.clone();
    let tx_ui_msg = tx_ui.clone();
    let tx_dc_clone = tx_dc.clone();
    let tx_chunks_inner = tx_chunks.clone();

    pc.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_clone = Arc::clone(&d);
        let pseudo_to_send = my_pseudo.clone();
        let target_msg_clone = target_id_msg.clone();
        let tx_ui_msg_clone = tx_ui_msg.clone();
        let tx_chunks_clone = tx_chunks_inner.clone();

        let _ = tx_dc_clone.send((target_msg_clone.clone(), Arc::clone(&d)));

        Box::pin(async move {
            let d_open = Arc::clone(&d_clone);
            let p_open = pseudo_to_send.clone();
            let tgt_open = target_msg_clone.clone();
            let tx_open = tx_ui_msg_clone.clone();

            d_clone.on_open(Box::new(move || {
                let p = p_open.clone();
                let tgt = tgt_open.clone();
                let tx = tx_open.clone();

                Box::pin(async move {
                    if is_call {
                        let _ = tx.send(format!("CALL_ACTIVE:{}", tgt));
                    }
                    let msg = format!("{{\"type\":\"pseudo\",\"value\":\"{}\"}}", p);
                    let _ = d_open.send_text(msg).await;
                })
            }));

            d_clone.on_message(Box::new(move |msg: DataChannelMessage| {
                let text = String::from_utf8_lossy(&msg.data);

                if text.starts_with("SYS:FILE_META:") || text.starts_with("SYS:ACK_META:") || text.starts_with("SYS:FILE_CHUNK:") || text.starts_with("SYS:ACK_CHUNK:") {
                    let _ = tx_chunks_clone.send((target_msg_clone.clone(), text.into_owned()));
                }
                else if text.starts_with("{\"type\":\"pseudo\"") {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(friend_pseudo) = json["value"].as_str() {
                            let _ = tx_ui_msg_clone.send(format!("CONTACT:{}:{}", target_msg_clone, friend_pseudo));
                            return Box::pin(async move {});
                        }
                    }
                } else {
                    let _ = tx_ui_msg_clone.send(format!("CHAT_RECV:{}:{}", target_msg_clone, text));
                }
                Box::pin(async move {})
            }));
        })
    }));

    Ok(pc)
}
