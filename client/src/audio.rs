use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

pub static REQUESTED_MIC: Mutex<Option<String>> = Mutex::new(None);
pub static REQUESTED_SPEAKER: Mutex<Option<String>> = Mutex::new(None);

pub fn set_microphone(name: String) {
    if let Ok(mut m) = REQUESTED_MIC.lock() {
        *m = Some(name);
    }
}

pub fn set_speaker(name: String) {
    if let Ok(mut m) = REQUESTED_SPEAKER.lock() {
        *m = Some(name);
    }
}

pub fn get_available_microphones() -> Vec<String> {
    let host = cpal::default_host();
    let mut list = vec!["Défaut".to_string()];
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let name_str = d.to_string();
            if !list.contains(&name_str) {
                list.push(name_str);
            }
        }
    }
    list
}

pub fn get_available_speakers() -> Vec<String> {
    let host = cpal::default_host();
    let mut list = vec!["Défaut".to_string()];
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            let name_str = d.to_string();
            if !list.contains(&name_str) {
                list.push(name_str);
            }
        }
    }
    list
}


pub fn detect_microphone() {
    let host = cpal::default_host();
    let _ = host.default_input_device();
}

pub fn start_hardware_audio(
    tx_mic: tokio::sync::mpsc::Sender<Vec<u8>>,
    rx_speaker: std::sync::mpsc::Receiver<(usize, Vec<i16>)>
) {
    std::thread::spawn(move || {
        let mut _mic_stream: Option<cpal::Stream> = None;
        let mut _spk_stream: Option<cpal::Stream> = None;
        let mut current_mic = String::new();
        let mut current_spk = String::new();
        
        // rx_speaker_arc moved top

        let rx_speaker_arc = Arc::new(Mutex::new(rx_speaker));
        loop {
            let req_mic = REQUESTED_MIC.lock().unwrap().clone().unwrap_or_else(|| "Défaut".to_string());
            let req_spk = REQUESTED_SPEAKER.lock().unwrap().clone().unwrap_or_else(|| "Défaut".to_string());
            
            let host = cpal::default_host();

        // ==========================================
        // 1. INITIALISATION DU MICRO (Encodeur Opus)
        // ==========================================
        let mic_device_opt = if req_mic == "Défaut" {
            host.default_input_device()
        } else {
            host.input_devices().unwrap().find(|d| d.to_string() == req_mic).or_else(|| host.default_input_device())
        };
        
        if req_mic != current_mic {
            current_mic = req_mic.clone();
            _mic_stream = None;

        if let Some(mic_device) = mic_device_opt {
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
        
        let _ = mic_stream.play();
        _mic_stream = Some(mic_stream);
        } // end of some(mic_device)
        } // end of req_mic != current_mic


        // ==========================================
        // 2. HAUT-PARLEURS (Le Mélangeur Audio)
        // ==========================================
        let spk_device_opt = if req_spk == "Défaut" {
            host.default_output_device()
        } else {
            host.output_devices().unwrap().find(|d| d.to_string() == req_spk).or_else(|| host.default_output_device())
        };
        
        if req_spk != current_spk {
            current_spk = req_spk.clone();
            _spk_stream = None;

        if let Some(spk_device) = spk_device_opt {
        let spk_supported_config = spk_device.default_output_config().unwrap();
        let spk_format = spk_supported_config.sample_format();
        let spk_config: cpal::StreamConfig = spk_supported_config.into();
        let spk_channels = spk_config.channels as usize;

        // rx_speaker_arc moved top
        let rx_spk_f32 = Arc::clone(&rx_speaker_arc);
        let rx_spk_i16 = Arc::clone(&rx_speaker_arc);

        let spk_stream = match spk_format {
            cpal::SampleFormat::F32 => {
                let mut track_buffers: std::collections::HashMap<usize, VecDeque<i16>> = std::collections::HashMap::new();
                                spk_device.build_output_stream(
                                    spk_config.clone(),
                                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {

                                        while let Ok((id, chunk)) = rx_spk_f32.lock().unwrap().try_recv() {
                                            let buf = track_buffers.entry(id).or_default();
                                            for val in chunk {
                                                buf.push_back(val);
                                            }
                                        }

                                        for buf in track_buffers.values_mut() {
                                            if buf.len() > 14400 {
                                                let excess = buf.len() - 14400;
                                                buf.drain(0..excess);
                                            }
                                        }

                                        for frame in data.chunks_mut(spk_channels) {
                                            let mut mixed_l = 0i32;
                                            let mut mixed_r = 0i32;

                                            for buf in track_buffers.values_mut() {
                                                let l = buf.pop_front().unwrap_or(0) as i32;
                                                let r = buf.pop_front().unwrap_or(0) as i32;
                                                mixed_l += l;
                                                mixed_r += r;
                                            }

                                            let l = mixed_l.clamp(i16::MIN as i32, i16::MAX as i32);
                                            let r = mixed_r.clamp(i16::MIN as i32, i16::MAX as i32);

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
                let mut track_buffers: std::collections::HashMap<usize, VecDeque<i16>> = std::collections::HashMap::new();
                                spk_device.build_output_stream(
                                    spk_config.clone(),
                                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {

                                        while let Ok((id, chunk)) = rx_spk_i16.lock().unwrap().try_recv() {
                                            let buf = track_buffers.entry(id).or_default();
                                            for val in chunk {
                                                buf.push_back(val);
                                            }
                                        }

                                        for buf in track_buffers.values_mut() {
                                            if buf.len() > 14400 {
                                                let excess = buf.len() - 14400;
                                                buf.drain(0..excess);
                                            }
                                        }

                                        for frame in data.chunks_mut(spk_channels) {
                                            let mut mixed_l = 0i32;
                                            let mut mixed_r = 0i32;

                                            for buf in track_buffers.values_mut() {
                                                let l = buf.pop_front().unwrap_or(0) as i32;
                                                let r = buf.pop_front().unwrap_or(0) as i32;
                                                mixed_l += l;
                                                mixed_r += r;
                                            }

                                            let l = mixed_l.clamp(i16::MIN as i32, i16::MAX as i32);
                                            let r = mixed_r.clamp(i16::MIN as i32, i16::MAX as i32);

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
        
        let _ = spk_stream.play();
        _spk_stream = Some(spk_stream);
        } // end of some(spk_device)
        } // end of req_spk != current_spk



            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });
}