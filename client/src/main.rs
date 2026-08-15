use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

// Importations WebRTC
use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;

// Importations Audio (cpal)
use cpal::traits::HostTrait;

// Importations pour l'audio WebRTC
use webrtc::track::track_local::track_local_static_sample::TrackLocalStaticSample;
use webrtc::rtp_transceiver::rtp_codec::RTCRtpCodecCapability;
use std::sync::Arc as StdArc;

// ==========================================
// STRUCTURE DES MESSAGES (JSON)
// ==========================================
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Signal {
    Offer { sdp: String },
    Answer { sdp: String },
    Ice { candidate: String },
    Ping { msg: String }
}

// ==========================================
// FONCTION DE DÉTECTION DU MICROPHONE
// ==========================================
fn detect_microphone() {
    println!("🔍 Recherche des périphériques audio...");
    let host = cpal::default_host();

    match host.default_input_device() {
        Some(_device) => {
            println!("🎤 Microphone par défaut détecté et prêt à l'emploi !");
        },
        None => {
            println!("❌ Aucun microphone n'a pu être trouvé ou autorisé !");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️ Lancement du client SecureP2P...");

    // Test matériel audio
    detect_microphone();

    // On vérifie si ce terminal a été lancé avec l'argument "--caller"
    let is_caller = std::env::args().any(|a| a == "--caller");

    // ==========================================
    // INITIALISATION DES CODECS MÉDIA (OPUS)
    // ==========================================
    let mut m = webrtc::api::media_engine::MediaEngine::default();
    m.register_default_codecs()?;
    let api = APIBuilder::new().with_media_engine(m).build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let peer_connection = Arc::new(api.new_peer_connection(config).await?);
    let (tx_signal, mut rx_signal) = tokio::sync::mpsc::channel::<Signal>(32);

    // ==========================================
    // ÉCOUTEUR D'ÉVÉNEMENTS ICE
    // ==========================================
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

    // ==========================================
    // RÉCEPTION DU FLUX AUDIO DISTANT
    // ==========================================
    peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
        Box::pin(async move {
            println!("🔊 [P2P] Flux audio distant détecté ! Réception de la voix en cours...");

            tokio::spawn(async move {
                let track = track;
                let mut buf = vec![0u8; 1500];
                while let Ok((_, _)) = track.read(&mut buf).await {
                    println!("🎧 Paquet audio distant reçu sur la piste !");
                }
            });
        })
    }));

    // ==========================================
    // CRÉATION DE LA PISTE AUDIO LOCALE (OPUS)
    // ==========================================
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

    // On ajoute cette piste au pont P2P
    let rtp_sender = peer_connection
        .add_track(StdArc::clone(&audio_track) as StdArc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>)
        .await?;

    // Maintenance des paquets RTCP en arrière-plan
    tokio::spawn(async move {
        let mut rtcp_buf = vec![0u8; 1500];
        while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
    });

    println!("🎤 Piste audio WebRTC (Opus) initialisée et attachée au pont !");

    // ==========================================
    // ÉVÉNEMENTS DU CANAL DE DONNÉES (P2P TEXTE)
    // ==========================================
    peer_connection.on_data_channel(Box::new(move |d: Arc<RTCDataChannel>| {
        let d_clone = Arc::clone(&d);
        Box::pin(async move {
            let d_open = Arc::clone(&d_clone);
            d_clone.on_open(Box::new(move || {
                println!("🎉 [P2P] PONT CONNECTÉ ! Sécurité maximale activée.");
                Box::pin(async move {
                    let _ = d_open.send_text("Hello Appelant ! Je te reçois fort et clair 5/5.").await;
                })
            }));

            d_clone.on_message(Box::new(move |msg: DataChannelMessage| {
                let text = String::from_utf8_lossy(&msg.data);
                println!("💬 [Message P2P Secret] : {}", text);
                Box::pin(async move {})
            }));
        })
    }));

    if is_caller {
        println!("📞 Mode Appelant activé !");
        let data_channel = peer_connection.create_data_channel("secure_text", None).await?;

        let d_open = Arc::clone(&data_channel);
        data_channel.on_open(Box::new(move || {
            println!("🎉 [P2P] PONT CONNECTÉ ! Sécurité maximale activée.");
            Box::pin(async move {
                let _ = d_open.send_text("Hello Appelé ! Voici mon premier message intraçable.").await;
            })
        }));

        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let text = String::from_utf8_lossy(&msg.data);
            println!("💬 [Message P2P Secret] : {}", text);
            Box::pin(async move {})
        }));

        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;
        let _ = tx_signal.send(Signal::Offer { sdp: offer.sdp }).await;
    } else {
        println!("⏳ Mode Écoute activé. En attente d'un appel...");
    }

    // ==========================================
    // SERVEUR DE SIGNALISATION & TRAITEMENT
    // ==========================================
    let url = "ws://127.0.0.1:8080";
    match connect_async(url).await {
        Ok((ws_stream, _)) => {
            println!("✅ Connecté au serveur de signalisation !");

            // ==========================================
            // CAPTURE MICROPHONE (CANAL SYNCHRone PUR)
            // ==========================================
            let audio_track_clone = Arc::clone(&audio_track);
            let (tx_audio, rx_audio) = std::sync::mpsc::channel::<Vec<u8>>();

            // Tâche asynchrone Tokio qui récupère les blocs et les écrit dans WebRTC
            tokio::spawn(async move {
                while let Ok(samples_u8) = rx_audio.recv() {
                    let sample_data = webrtc::media::Sample {
                        data: bytes::Bytes::from(samples_u8),
                        duration: std::time::Duration::from_millis(20),
                        ..Default::default()
                    };
                    let _ = audio_track_clone.write_sample(&sample_data).await;
                }
            });

            // Thread natif pur pour CPAL (sans aucun appel au contexte Tokio)
            std::thread::spawn(move || {
                use cpal::traits::{DeviceTrait, StreamTrait};
                let host = cpal::default_host();
                let device = match host.default_input_device() {
                    Some(d) => d,
                    None => return,
                };
                let config = match device.default_input_config() {
                    Ok(c) => c,
                    Err(_) => return,
                };

                println!("🎙️ Démarrage de la capture audio en direct du microphone...");

                let err_fn = move |_err| {};
                let tx_audio_clone = tx_audio;

                if let Ok(stream) = device.build_input_stream(
                    config.into(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let mut samples_u8 = Vec::with_capacity(data.len() * 2);
                        for &sample in data {
                            samples_u8.extend_from_slice(&sample.to_le_bytes());
                        }
                        let _ = tx_audio_clone.send(samples_u8);
                    },
                    err_fn,
                    None,
                ) {
                    let _ = stream.play();
                }

                std::thread::park();
            });

            let (mut ws_sender, mut ws_receiver) = ws_stream.split();

            tokio::spawn(async move {
                while let Some(msg) = rx_signal.recv().await {
                    if let Ok(json_msg) = serde_json::to_string(&msg) {
                        let _ = ws_sender.send(Message::Text(json_msg.into())).await;
                    }
                }
            });

            let tx_ws = tx_signal.clone();

            while let Some(Ok(response)) = ws_receiver.next().await {
                if let Ok(text) = response.into_text() {
                    if let Ok(signal) = serde_json::from_str::<Signal>(&text) {
                        match signal {
                            Signal::Offer { sdp } => {
                                if is_caller { continue; }

                                println!("📥 [Signal] Offre SDP reçue !");
                                let mut desc = RTCSessionDescription::default();
                                desc.sdp_type = RTCSdpType::Offer;
                                desc.sdp = sdp;

                                if peer_connection.set_remote_description(desc).await.is_ok() {
                                    if let Ok(answer) = peer_connection.create_answer(None).await {
                                        if peer_connection.set_local_description(answer.clone()).await.is_ok() {
                                            let _ = tx_ws.send(Signal::Answer { sdp: answer.sdp }).await;
                                            println!("🚀 Réponse SDP (Answer) envoyée !");
                                        }
                                    }
                                }
                            },
                            Signal::Answer { sdp } => {
                                if !is_caller { continue; }

                                println!("📥 [Signal] Réponse SDP reçue !");
                                let mut desc = RTCSessionDescription::default();
                                desc.sdp_type = RTCSdpType::Answer;
                                desc.sdp = sdp;
                                let _ = peer_connection.set_remote_description(desc).await;
                            },
                            Signal::Ice { candidate } => {
                                if peer_connection.remote_description().await.is_some() {
                                    let ice_init = RTCIceCandidateInit {
                                        candidate,
                                        ..Default::default()
                                    };
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