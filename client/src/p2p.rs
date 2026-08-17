use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use std::sync::Arc as StdArc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Signal { Offer { sdp: String }, Answer { sdp: String }, Ice { candidate: String }, Ping { _msg: String } }

pub async fn start_p2p(
    mut rx_mic: tokio::sync::mpsc::Receiver<Vec<u8>>,
    tx_speaker: std::sync::mpsc::Sender<Vec<i16>>,
    mut rx_ui: UnboundedReceiver<String>,
    tx_ui: UnboundedSender<String>, // NOTRE NOUVEAU CÂBLE VERS L'UI !
) -> Result<(), Box<dyn std::error::Error>> {

    let mut m = webrtc::api::media_engine::MediaEngine::default();
    m.register_default_codecs()?;
    let api = APIBuilder::new().with_media_engine(m).build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer { urls: vec!["stun:stun.l.google.com:19302".to_owned()], ..Default::default() }],
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await?);
    let (tx_signal, mut rx_signal) = tokio::sync::mpsc::channel::<Signal>(32);

    let tx_ice = tx_signal.clone();
    peer_connection.on_ice_candidate(Box::new(move |c: Option<RTCIceCandidate>| {
        let tx_ice = tx_ice.clone();
        Box::pin(async move {
            if let Some(candidate) = c {
                if let Ok(json) = candidate.to_json() {
                    let _ = tx_ice.send(Signal::Ice { candidate: json.candidate }).await;
                }
            }
        })
    }));

    let tx_ui_audio = tx_ui.clone();
    peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
        let tx_speaker = tx_speaker.clone();
        let tx_ui_audio = tx_ui_audio.clone();
        Box::pin(async move {
            println!("🔊 [P2P] Flux audio distant détecté !");
            let _ = tx_ui_audio.send("🔊 Appel en cours ! Pont vocal sécurisé actif.".to_string());

            tokio::spawn(async move {
                let mut decoder = audiopus::coder::Decoder::new(audiopus::SampleRate::Hz48000, audiopus::Channels::Stereo).unwrap();
                let track = track;
                while let Ok((rtp_packet, _)) = track.read_rtp().await {
                    let mut decoded_pcm = vec![0i16; 1920 * 2];
                    if let Ok(len) = decoder.decode(Some(rtp_packet.payload.as_ref()), &mut decoded_pcm, false) {
                        let total_samples = len * 2;
                        decoded_pcm.truncate(total_samples);
                        let _ = tx_speaker.send(decoded_pcm);
                    }
                }
            });
        })
    }));

    let audio_track = StdArc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability { mime_type: "audio/opus".to_owned(), clock_rate: 48000, channels: 2, ..Default::default() },
        "audio_p2p".to_owned(), "kakolookiyam_voice".to_owned(),
    ));

    let rtp_sender = peer_connection.add_track(StdArc::clone(&audio_track) as StdArc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>).await?;
    tokio::spawn(async move { let mut rtcp_buf = vec![0u8; 1500]; while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {} });

    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_clone = Arc::clone(&d);
        Box::pin(async move {
            let d_open = Arc::clone(&d_clone);
            d_clone.on_open(Box::new(move || {
                Box::pin(async move { let _ = d_open.send_text("Hello ! J'ai bien reçu ton appel.").await; })
            }));
            d_clone.on_message(Box::new(move |msg: DataChannelMessage| {
                println!("💬 [Message] : {}", String::from_utf8_lossy(&msg.data)); Box::pin(async move {})
            }));
        })
    }));

    let pc_clone = Arc::clone(&peer_connection);
    let tx_sig_clone = tx_signal.clone();
    tokio::spawn(async move {
        while let Some(peer_id) = rx_ui.recv().await {
            if peer_id.trim().is_empty() { continue; }
            if let Ok(data_channel) = pc_clone.create_data_channel("secure_text", None).await {
                let d_open = Arc::clone(&data_channel);
                data_channel.on_open(Box::new(move || {
                    Box::pin(async move { let _ = d_open.send_text("Hello ! J'ai lancé l'appel depuis l'interface.").await; })
                }));
            }
            if let Ok(offer) = pc_clone.create_offer(None).await {
                if pc_clone.set_local_description(offer.clone()).await.is_ok() {
                    let _ = tx_sig_clone.send(Signal::Offer { sdp: offer.sdp }).await;
                }
            }
        }
    });

    let url = "ws://127.0.0.1:8080";
    match connect_async(url).await {
        Ok((ws_stream, _)) => {
            println!("✅ Connecté au serveur de signalisation en attente d'actions...");

            let audio_track_clone = Arc::clone(&audio_track);
            tokio::spawn(async move {
                while let Some(opus_packet) = rx_mic.recv().await {
                    let sample_data = webrtc::media::Sample { data: bytes::Bytes::from(opus_packet), duration: std::time::Duration::from_millis(20), ..Default::default() };
                    let _ = audio_track_clone.write_sample(&sample_data).await;
                }
            });

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();
            tokio::spawn(async move {
                while let Some(msg) = rx_signal.recv().await {
                    if let Ok(json_msg) = serde_json::to_string(&msg) { let _ = ws_sender.send(Message::Text(json_msg.into())).await; }
                }
            });

            let tx_ws = tx_signal.clone();
            let tx_ui_ws = tx_ui.clone();

            while let Some(Ok(response)) = ws_receiver.next().await {
                if let Ok(text) = response.into_text() {
                    if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                        match signal {
                            Signal::Offer { sdp } => {
                                println!("📥 Offre d'appel reçue !");
                                // Avertissement UI immédiat
                                let _ = tx_ui_ws.send("📥 Appel entrant reçu ! Décrochage en cours...".to_string());

                                let mut desc = RTCSessionDescription::default();
                                desc.sdp_type = RTCSdpType::Offer; desc.sdp = sdp;
                                if peer_connection.set_remote_description(desc).await.is_ok() {
                                    if let Ok(answer) = peer_connection.create_answer(None).await {
                                        if peer_connection.set_local_description(answer.clone()).await.is_ok() {
                                            let _ = tx_ws.send(Signal::Answer { sdp: answer.sdp }).await;
                                        }
                                    }
                                }
                            },
                            Signal::Answer { sdp } => {
                                println!("📥 Réponse acceptée !");
                                let _ = tx_ui_ws.send("🚀 L'ami a décroché ! Négociation...".to_string());
                                let mut desc = RTCSessionDescription::default();
                                desc.sdp_type = RTCSdpType::Answer; desc.sdp = sdp;
                                let _ = peer_connection.set_remote_description(desc).await;
                            },
                            Signal::Ice { candidate } => {
                                if peer_connection.remote_description().await.is_some() {
                                    let ice_init = RTCIceCandidateInit { candidate, ..Default::default() };
                                    let _ = peer_connection.add_ice_candidate(ice_init).await;
                                }
                            },
                            Signal::Ping { .. } => {}
                        }
                    }
                }
            }
        },
        Err(e) => println!("❌ Impossible de se connecter au serveur : {}", e)
    }
    Ok(())
}