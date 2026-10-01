use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
pub static REQUESTED_MIC: Mutex<Option<String>> = Mutex::new(None);
pub static REQUESTED_SPEAKER: Mutex<Option<String>> = Mutex::new(None);
pub static AUDIO_THREAD: Mutex<Option<std::thread::Thread>> = Mutex::new(None);

pub static USER_VOLUMES: Mutex<Option<std::collections::HashMap<String, f32>>> = Mutex::new(None);

pub fn get_user_volume(id: &str) -> f32 {
    if let Ok(m) = USER_VOLUMES.lock() {
        if let Some(map) = &*m {
            return *map.get(id).unwrap_or(&1.0);
        }
    }
    1.0
}

pub fn set_user_volume(id: String, volume: f32) {
    if let Ok(mut m) = USER_VOLUMES.lock() {
        if m.is_none() {
            *m = Some(std::collections::HashMap::new());
        }
        if let Some(map) = m.as_mut() {
            map.insert(id, volume);
        }
    }
}


pub fn set_microphone(name: String) {
    if let Ok(mut m) = REQUESTED_MIC.lock() {
        *m = Some(name);
    }
    if let Ok(t) = AUDIO_THREAD.lock() {
        if let Some(thread) = &*t { thread.unpark(); }
    }
}

pub fn set_speaker(name: String) {
    if let Ok(mut m) = REQUESTED_SPEAKER.lock() {
        *m = Some(name);
    }
    if let Ok(t) = AUDIO_THREAD.lock() {
        if let Some(thread) = &*t { thread.unpark(); }
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
    let handle = std::thread::spawn(move || {
        let rx_speaker_arc = std::sync::Arc::new(std::sync::Mutex::new(rx_speaker));
        loop {
            let tx_mic = tx_mic.clone();
            let host = cpal::default_host();
        
                let mut mic_device = host.default_input_device();
        if let Ok(m) = crate::audio::REQUESTED_MIC.lock() {
            if let Some(name) = &*m {
                if name != "Défaut" && name != "Dfaut" {
                    if let Ok(devices) = host.input_devices() {
                        for d in devices {
                            if d.to_string() == *name {
                                mic_device = Some(d);
                                break;
                            }
                        }
                    }
                }
            }
        }
        let mic_device = match mic_device {
            Some(d) => d,
            None => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };
        let mic_config_supported = match mic_device.default_input_config() {
            Ok(c) => c,
            Err(_) => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };
        let mut mic_config: cpal::StreamConfig = mic_config_supported.clone().into();
        mic_config.sample_rate = 48000;
        let mic_channels = mic_config.channels as usize;

        let opus_channels = audiopus::Channels::Mono;
        let chunk_size = 960;

        let tx_mic_f32 = tx_mic.clone();
        let tx_mic_i16 = tx_mic.clone();

        let mic_stream_res = match mic_config_supported.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut encoder = match audiopus::coder::Encoder::new(audiopus::SampleRate::Hz48000, opus_channels, audiopus::Application::Voip) {
                    Ok(e) => e,
                    Err(_) => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
                };
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
                            let mut frame: Vec<i16> = mic_buffer.drain(..chunk_size).collect();
                            crate::reduction::process_chunk(&mut frame); let mut encoded = vec![0u8; 1500];
                            if let Ok(len) = encoder.encode(&frame, &mut encoded) {
                                encoded.truncate(len);
                                let _ = tx_mic_f32.blocking_send(encoded);
                            }
                        }
                    },
                    |_err| {},
                    None,
                )
            },
            cpal::SampleFormat::I16 => {
                let mut encoder = match audiopus::coder::Encoder::new(audiopus::SampleRate::Hz48000, opus_channels, audiopus::Application::Voip) {
                    Ok(e) => e,
                    Err(_) => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
                };
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
                            let mut frame: Vec<i16> = mic_buffer.drain(..chunk_size).collect();
                            crate::reduction::process_chunk(&mut frame); let mut encoded = vec![0u8; 1500];
                            if let Ok(len) = encoder.encode(&frame, &mut encoded) {
                                encoded.truncate(len);
                                let _ = tx_mic_i16.blocking_send(encoded);
                            }
                        }
                    },
                    |_err| {},
                    None,
                )
            },
            _ => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };

        let mic_stream = match mic_stream_res {
            Ok(s) => s,
            Err(_) => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };

                let mut spk_device = host.default_output_device();
        if let Ok(m) = crate::audio::REQUESTED_SPEAKER.lock() {
            if let Some(name) = &*m {
                if name != "Défaut" && name != "Dfaut" {
                    if let Ok(devices) = host.output_devices() {
                        for d in devices {
                            if d.to_string() == *name {
                                spk_device = Some(d);
                                break;
                            }
                        }
                    }
                }
            }
        }
        let spk_device = match spk_device {
            Some(d) => d,
            None => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };
        let spk_supported_config = match spk_device.default_output_config() {
            Ok(c) => c,
            Err(_) => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };
        let spk_format = spk_supported_config.sample_format();
        let mut spk_config: cpal::StreamConfig = spk_supported_config.into();
        spk_config.sample_rate = 48000;
        let spk_channels = spk_config.channels as usize;

        let rx_spk_f32 = Arc::clone(&rx_speaker_arc);
        let rx_spk_i16 = Arc::clone(&rx_speaker_arc);

        let spk_stream_res = match spk_format {
            cpal::SampleFormat::F32 => {
                let mut track_buffers: std::collections::HashMap<usize, VecDeque<i16>> = std::collections::HashMap::new();
                spk_device.build_output_stream(
                    spk_config.clone(),
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        while let Ok((id, chunk)) = rx_spk_f32.lock().unwrap().try_recv() {
                            let buf = track_buffers.entry(id).or_default();
                            for val in chunk { buf.push_back(val); }
                        }
                        for buf in track_buffers.values_mut() {
                            if buf.len() > 14400 { let excess = buf.len() - 14400; buf.drain(0..excess); }
                        }
                        for frame in data.chunks_mut(spk_channels) {
                            let mut mixed_l = 0i32; let mut mixed_r = 0i32;
                            for buf in track_buffers.values_mut() {
                                mixed_l += buf.pop_front().unwrap_or(0) as i32;
                                mixed_r += buf.pop_front().unwrap_or(0) as i32;
                            }
                            let l = mixed_l.clamp(i16::MIN as i32, i16::MAX as i32);
                            let r = mixed_r.clamp(i16::MIN as i32, i16::MAX as i32);
                            let l_f32 = l as f32 / i16::MAX as f32;
                            let r_f32 = r as f32 / i16::MAX as f32;
                            if spk_channels == 1 {
                                frame[0] = (l_f32 + r_f32) / 2.0;
                            } else if spk_channels >= 2 {
                                frame[0] = l_f32; frame[1] = r_f32;
                                for s in frame.iter_mut().skip(2) { *s = 0.0; }
                            }
                        }
                        track_buffers.retain(|_, buf| !buf.is_empty());
                    },
                    |_err| {},
                    None,
                )
            },
            cpal::SampleFormat::I16 => {
                let mut track_buffers: std::collections::HashMap<usize, VecDeque<i16>> = std::collections::HashMap::new();
                spk_device.build_output_stream(
                    spk_config.clone(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        while let Ok((id, chunk)) = rx_spk_i16.lock().unwrap().try_recv() {
                            let buf = track_buffers.entry(id).or_default();
                            for val in chunk { buf.push_back(val); }
                        }
                        for buf in track_buffers.values_mut() {
                            if buf.len() > 14400 { let excess = buf.len() - 14400; buf.drain(0..excess); }
                        }
                        for frame in data.chunks_mut(spk_channels) {
                            let mut mixed_l = 0i32; let mut mixed_r = 0i32;
                            for buf in track_buffers.values_mut() {
                                mixed_l += buf.pop_front().unwrap_or(0) as i32;
                                mixed_r += buf.pop_front().unwrap_or(0) as i32;
                            }
                            let l = mixed_l.clamp(i16::MIN as i32, i16::MAX as i32);
                            let r = mixed_r.clamp(i16::MIN as i32, i16::MAX as i32);
                            if spk_channels == 1 {
                                frame[0] = ((l + r) / 2) as i16;
                            } else if spk_channels >= 2 {
                                frame[0] = l as i16; frame[1] = r as i16;
                                for s in frame.iter_mut().skip(2) { *s = 0; }
                            }
                        }
                        track_buffers.retain(|_, buf| !buf.is_empty());
                    },
                    |_err| {},
                    None,
                )
            },
            _ => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };

        let spk_stream = match spk_stream_res {
            Ok(s) => s,
            Err(_) => { std::thread::sleep(std::time::Duration::from_millis(1000)); continue; }
        };

        let _ = mic_stream.play();
        let _ = spk_stream.play();

        std::thread::park();
        }
    });
    if let Ok(mut t) = AUDIO_THREAD.lock() {
        *t = Some(handle.thread().clone());
    }
}

