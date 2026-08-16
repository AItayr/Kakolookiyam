#![allow(unused_mut)] // Masque les avertissements inoffensifs du compilateur

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

    detect_microphone();
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
    // CANAL DE COMMUNICATION POUR LES HAUT-PARLEURS
    // ==========================================
    let (tx_speaker, rx_speaker) = std::sync::mpsc::channel::<Vec<i16>>();

    peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
        let tx_speaker = tx_speaker.clone();
        Box::pin(async move {
            println!("🔊 [P2P] Flux audio distant détecté ! Réception de la voix en cours...");

            tokio::spawn(async move {
                // Le décodeur Opus recrache en Stéréo (2 canaux centrés si la source est Mono)
                let mut decoder = audiopus::coder::Decoder::new(
                    audiopus::SampleRate::Hz48000,
                    audiopus::Channels::Stereo
                ).expect("Erreur création décodeur Opus");

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
        RTCRtpCodecCapability {
            mime_type: "audio/opus".to_owned(),
            clock_rate: 48000,
            channels: 2,
            ..Default::default()
        },
        "audio_p2p".to_owned(),
        "kakolookiyam_voice".to_owned(),
    ));

    let rtp_sender = peer_connection
        .add_track(StdArc::clone(&audio_track) as StdArc<dyn webrtc::track::track_local::TrackLocal + Send + Sync>)
        .await?;

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

            let audio_track_clone = Arc::clone(&audio_track);
            let (tx_audio, mut rx_audio) = tokio::sync::mpsc::channel::<Vec<u8>>(500);

            tokio::spawn(async move {
                while let Some(opus_packet) = rx_audio.recv().await {
                    let sample_data = webrtc::media::Sample {
                        data: bytes::Bytes::from(opus_packet),
                        duration: std::time::Duration::from_millis(20),
                        ..Default::default()
                    };
                    let _ = audio_track_clone.write_sample(&sample_data).await;
                }
            });

            std::thread::spawn(move || {
                use cpal::traits::{DeviceTrait, StreamTrait};
                let host = cpal::default_host();

                // 1. Initialisation du MICROPHONE
                let mic_device = match host.default_input_device() {
                    Some(d) => d,
                    None => return,
                };
                let mic_supported_config = match mic_device.default_input_config() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let mic_config: cpal::StreamConfig = mic_supported_config.into();
                let mic_channels = mic_config.channels as usize;

                println!("🎙️ Démarrage de la capture audio en direct du microphone...");

                let tx_audio_f32 = tx_audio.clone();
                let tx_audio_i16 = tx_audio.clone();
                let err_fn_mic = |err| eprintln!("⚠️ Erreur micro : {}", err);

                // CORRECTION : On FORCE l'encodeur en MONO (recentrage parfait de la voix)
                let opus_channels = audiopus::Channels::Mono;
                let chunk_size = 960; // 20ms à 48kHz en Mono = exactement 960 échantillons

                let create_encoder = || {
                    audiopus::coder::Encoder::new(
                        audiopus::SampleRate::Hz48000,
                        opus_channels,
                        audiopus::Application::Voip
                    ).expect("Erreur création encodeur Opus")
                };

                let mic_stream_opt = match mic_supported_config.sample_format() {
                    cpal::SampleFormat::F32 => {
                        let mut encoder = create_encoder();
                        let mut mic_buffer = Vec::new();
                        mic_device.build_input_stream(
                            mic_config.clone(),
                            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                                // DOWNMIX : On additionne les canaux de la carte son (G + D)
                                for frame in data.chunks(mic_channels) {
                                    let sum: f32 = frame.iter().sum();
                                    let mono_sample = sum.clamp(-1.0, 1.0); // Anti-saturation
                                    mic_buffer.push((mono_sample * i16::MAX as f32) as i16);
                                }

                                while mic_buffer.len() >= chunk_size {
                                    let frame: Vec<i16> = mic_buffer.drain(..chunk_size).collect();
                                    let mut encoded = vec![0u8; 1500];
                                    if let Ok(len) = encoder.encode(&frame, &mut encoded) {
                                        encoded.truncate(len);
                                        let _ = tx_audio_f32.blocking_send(encoded);
                                    }
                                }
                            },
                            err_fn_mic,
                            None,
                        ).ok()
                    },
                    cpal::SampleFormat::I16 => {
                        let mut encoder = create_encoder();
                        let mut mic_buffer = Vec::new();
                        mic_device.build_input_stream(
                            mic_config.clone(),
                            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                                // DOWNMIX : On additionne les canaux de la carte son (G + D)
                                for frame in data.chunks(mic_channels) {
                                    let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                                    let mono_sample = sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                                    mic_buffer.push(mono_sample);
                                }

                                while mic_buffer.len() >= chunk_size {
                                    let frame: Vec<i16> = mic_buffer.drain(..chunk_size).collect();
                                    let mut encoded = vec![0u8; 1500];
                                    if let Ok(len) = encoder.encode(&frame, &mut encoded) {
                                        encoded.truncate(len);
                                        let _ = tx_audio_i16.blocking_send(encoded);
                                    }
                                }
                            },
                            err_fn_mic,
                            None,
                        ).ok()
                    },
                    _ => None,
                };

                let mic_stream = match mic_stream_opt {
                    Some(s) => s,
                    None => return,
                };

                // 2. Initialisation des HAUT-PARLEURS
                let spk_device = match host.default_output_device() {
                    Some(d) => d,
                    None => return,
                };
                let spk_supported_config = match spk_device.default_output_config() {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let spk_config: cpal::StreamConfig = spk_supported_config.into();
                let spk_channels = spk_config.channels as usize;

                println!("🔈 Initialisation des haut-parleurs...");

                let rx_speaker_arc = std::sync::Arc::new(std::sync::Mutex::new(rx_speaker));
                let rx_spk_f32 = std::sync::Arc::clone(&rx_speaker_arc);
                let rx_spk_i16 = std::sync::Arc::clone(&rx_speaker_arc);
                let err_fn_spk = |err| eprintln!("⚠️ Erreur haut-parleur : {}", err);

                let spk_stream_opt = match spk_supported_config.sample_format() {
                    cpal::SampleFormat::F32 => {
                        let mut spk_buffer: std::collections::VecDeque<i16> = std::collections::VecDeque::new();
                        spk_device.build_output_stream(
                            spk_config.clone(),
                            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                                while let Ok(pcm_chunk) = rx_spk_f32.lock().unwrap().try_recv() {
                                    spk_buffer.extend(pcm_chunk);
                                }

                                // ANTI-DÉLAI
                                if spk_buffer.len() > 14400 {
                                    let excess = spk_buffer.len() - 14400;
                                    spk_buffer.drain(0..excess);
                                }

                                for frame in data.chunks_mut(spk_channels) {
                                    let l = spk_buffer.pop_front().unwrap_or(0);
                                    let r = spk_buffer.pop_front().unwrap_or(0);

                                    let l_f32 = l as f32 / i16::MAX as f32;
                                    let r_f32 = r as f32 / i16::MAX as f32;

                                    if spk_channels == 1 {
                                        frame[0] = (l_f32 + r_f32) / 2.0;
                                    } else if spk_channels >= 2 {
                                        frame[0] = l_f32; // Oreille gauche
                                        frame[1] = r_f32; // Oreille droite
                                        for s in frame.iter_mut().skip(2) { *s = 0.0; }
                                    }
                                }
                            },
                            err_fn_spk,
                            None,
                        ).ok()
                    },
                    cpal::SampleFormat::I16 => {
                        let mut spk_buffer: std::collections::VecDeque<i16> = std::collections::VecDeque::new();
                        spk_device.build_output_stream(
                            spk_config.clone(),
                            move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                                while let Ok(pcm_chunk) = rx_spk_i16.lock().unwrap().try_recv() {
                                    spk_buffer.extend(pcm_chunk);
                                }

                                // ANTI-DÉLAI
                                if spk_buffer.len() > 14400 {
                                    let excess = spk_buffer.len() - 14400;
                                    spk_buffer.drain(0..excess);
                                }

                                for frame in data.chunks_mut(spk_channels) {
                                    let l = spk_buffer.pop_front().unwrap_or(0);
                                    let r = spk_buffer.pop_front().unwrap_or(0);

                                    if spk_channels == 1 {
                                        frame[0] = ((l as i32 + r as i32) / 2) as i16;
                                    } else if spk_channels >= 2 {
                                        frame[0] = l;
                                        frame[1] = r;
                                        for s in frame.iter_mut().skip(2) { *s = 0; }
                                    }
                                }
                            },
                            err_fn_spk,
                            None,
                        ).ok()
                    },
                    _ => None,
                };

                let _ = mic_stream.play();
                if let Some(s) = &spk_stream_opt {
                    let _ = s.play();
                    println!("🔈 Lecture audio sur les haut-parleurs PRÊTE !");
                }

                std::thread::park();
            });

            // Boucle principale WebSocket
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
                                let mut desc = RTCSessionDescription::default();
                                desc.sdp_type = RTCSdpType::Offer;
                                desc.sdp = sdp;
                                if peer_connection.set_remote_description(desc).await.is_ok() {
                                    if let Ok(answer) = peer_connection.create_answer(None).await {
                                        if peer_connection.set_local_description(answer.clone()).await.is_ok() {
                                            let _ = tx_ws.send(Signal::Answer { sdp: answer.sdp }).await;
                                        }
                                    }
                                }
                            },
                            Signal::Answer { sdp } => {
                                if !is_caller { continue; }
                                let mut desc = RTCSessionDescription::default();
                                desc.sdp_type = RTCSdpType::Answer;
                                desc.sdp = sdp;
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