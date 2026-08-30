use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

pub fn detect_microphone() {
    let host = cpal::default_host();
    let _ = host.default_input_device();
}

pub fn start_hardware_audio(
    tx_mic: tokio::sync::mpsc::Sender<Vec<u8>>,
    rx_speaker: std::sync::mpsc::Receiver<Vec<i16>>
) {
    std::thread::spawn(move || {
        let host = cpal::default_host();

        // ==========================================
        // 1. INITIALISATION DU MICRO (Encodeur Opus)
        // ==========================================
        let mic_device = host.default_input_device().expect("Aucun micro trouvé");
        let mic_config: cpal::StreamConfig = mic_device.default_input_config().unwrap().into();
        let mic_channels = mic_config.channels as usize;

        let opus_channels = audiopus::Channels::Mono;
        let chunk_size = 960;

        let tx_mic_f32 = tx_mic.clone();
        let tx_mic_i16 = tx_mic.clone();

        let mic_stream = match mic_device.default_input_config().unwrap().sample_format() {
            cpal::SampleFormat::F32 => {
                let mut encoder = audiopus::coder::Encoder::new(
                    audiopus::SampleRate::Hz48000,
                    opus_channels,
                    audiopus::Application::Voip
                ).unwrap();
                let mut mic_buffer = Vec::new();

                mic_device.build_input_stream(
                    mic_config.clone(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        for frame in data.chunks(mic_channels) {
                            let sum: f32 = frame.iter().sum();
                            let mono_sample = (sum / mic_channels as f32).clamp(-1.0, 1.0);
                            mic_buffer.push((mono_sample * i16::MAX as f32) as i16);
                        }

                        while mic_buffer.len() >= chunk_size {
                            let frame: Vec<i16> = mic_buffer.drain(..chunk_size).collect();
                            let mut encoded = vec![0u8; 1500];

                            if let Ok(len) = encoder.encode(&frame, &mut encoded) {
                                encoded.truncate(len);
                                let _ = tx_mic_f32.blocking_send(encoded);
                            }
                        }
                    },
                    |_err| {},
                    None,
                ).unwrap()
            },
            cpal::SampleFormat::I16 => {
                let mut encoder = audiopus::coder::Encoder::new(
                    audiopus::SampleRate::Hz48000,
                    opus_channels,
                    audiopus::Application::Voip
                ).unwrap();
                let mut mic_buffer = Vec::new();

                mic_device.build_input_stream(
                    mic_config.clone(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        for frame in data.chunks(mic_channels) {
                            let sum: i32 = frame.iter().map(|&s| s as i32).sum();
                            let mono_sample = (sum / mic_channels as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
                            mic_buffer.push(mono_sample);
                        }

                        while mic_buffer.len() >= chunk_size {
                            let frame: Vec<i16> = mic_buffer.drain(..chunk_size).collect();
                            let mut encoded = vec![0u8; 1500];

                            if let Ok(len) = encoder.encode(&frame, &mut encoded) {
                                encoded.truncate(len);
                                let _ = tx_mic_i16.blocking_send(encoded);
                            }
                        }
                    },
                    |_err| {},
                    None,
                ).unwrap()
            },
            _ => panic!("Format micro non supporté"),
        };

        // ==========================================
        // 2. HAUT-PARLEURS (Le Mélangeur Audio)
        // ==========================================
        let spk_device = host.default_output_device().expect("Aucun haut-parleur trouvé");
        let spk_supported_config = spk_device.default_output_config().unwrap();
        let spk_format = spk_supported_config.sample_format();
        let spk_config: cpal::StreamConfig = spk_supported_config.into();
        let spk_channels = spk_config.channels as usize;

        let rx_speaker_arc = Arc::new(Mutex::new(rx_speaker));
        let rx_spk_f32 = Arc::clone(&rx_speaker_arc);
        let rx_spk_i16 = Arc::clone(&rx_speaker_arc);

        let spk_stream = match spk_format {
            cpal::SampleFormat::F32 => {
                let mut master_buffer: VecDeque<i32> = VecDeque::new();

                spk_device.build_output_stream(
                    spk_config.clone(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {

                        let mut packets = Vec::new();
                        while let Ok(chunk) = rx_spk_f32.lock().unwrap().try_recv() {
                            packets.push(chunk);
                        }

                        if !packets.is_empty() {
                            let max_len = packets.iter().map(|p| p.len()).max().unwrap_or(0);
                            let mut mixed = vec![0i32; max_len];

                            for packet in packets {
                                for (i, &sample) in packet.iter().enumerate() {
                                    mixed[i] += sample as i32;
                                }
                            }

                            for val in mixed {
                                master_buffer.push_back(val);
                            }
                        }

                        if master_buffer.len() > 14400 {
                            let excess = master_buffer.len() - 14400;
                            master_buffer.drain(0..excess);
                        }

                        for frame in data.chunks_mut(spk_channels) {
                            let mut l = master_buffer.pop_front().unwrap_or(0);
                            let mut r = master_buffer.pop_front().unwrap_or(0);

                            l = l.clamp(i16::MIN as i32, i16::MAX as i32);
                            r = r.clamp(i16::MIN as i32, i16::MAX as i32);

                            let l_f32 = l as f32 / i16::MAX as f32;
                            let r_f32 = r as f32 / i16::MAX as f32;

                            if spk_channels == 1 {
                                frame[0] = (l_f32 + r_f32) / 2.0;
                            } else if spk_channels >= 2 {
                                frame[0] = l_f32;
                                frame[1] = r_f32;
                                for s in frame.iter_mut().skip(2) {
                                    *s = 0.0;
                                }
                            }
                        }
                    },
                    |_err| {},
                    None,
                ).unwrap()
            },
            cpal::SampleFormat::I16 => {
                let mut master_buffer: VecDeque<i32> = VecDeque::new();

                spk_device.build_output_stream(
                    spk_config.clone(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {

                        let mut packets = Vec::new();
                        while let Ok(chunk) = rx_spk_i16.lock().unwrap().try_recv() {
                            packets.push(chunk);
                        }

                        if !packets.is_empty() {
                            let max_len = packets.iter().map(|p| p.len()).max().unwrap_or(0);
                            let mut mixed = vec![0i32; max_len];

                            for packet in packets {
                                for (i, &sample) in packet.iter().enumerate() {
                                    mixed[i] += sample as i32;
                                }
                            }

                            for val in mixed {
                                master_buffer.push_back(val);
                            }
                        }

                        if master_buffer.len() > 14400 {
                            let excess = master_buffer.len() - 14400;
                            master_buffer.drain(0..excess);
                        }

                        for frame in data.chunks_mut(spk_channels) {
                            let mut l = master_buffer.pop_front().unwrap_or(0);
                            let mut r = master_buffer.pop_front().unwrap_or(0);

                            l = l.clamp(i16::MIN as i32, i16::MAX as i32);
                            r = r.clamp(i16::MIN as i32, i16::MAX as i32);

                            if spk_channels == 1 {
                                frame[0] = ((l + r) / 2) as i16;
                            } else if spk_channels >= 2 {
                                frame[0] = l as i16;
                                frame[1] = r as i16;
                                for s in frame.iter_mut().skip(2) {
                                    *s = 0;
                                }
                            }
                        }
                    },
                    |_err| {},
                    None,
                ).unwrap()
            },
            _ => panic!("Format audio non supporté"),
        };

        let _ = mic_stream.play();
        let _ = spk_stream.play();

        std::thread::park();
    });
}