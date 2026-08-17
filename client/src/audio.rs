use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub fn detect_microphone() {
    println!("🔍 Recherche des périphériques audio...");
    let host = cpal::default_host();

    match host.default_input_device() {
        Some(_device) => println!("🎤 Microphone par défaut détecté et prêt à l'emploi !"),
        None => println!("❌ Aucun microphone n'a pu être trouvé ou autorisé !"),
    }
}

pub fn start_hardware_audio(
    tx_audio: tokio::sync::mpsc::Sender<Vec<u8>>,
    rx_speaker: std::sync::mpsc::Receiver<Vec<i16>>,
) {
    std::thread::spawn(move || {
        let host = cpal::default_host();

        // ==========================================
        // 1. INITIALISATION DU MICROPHONE
        // ==========================================
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

        let opus_channels = audiopus::Channels::Mono;
        let chunk_size = 960; // 20ms à 48kHz en Mono = 960 échantillons

        let create_encoder = || {
            audiopus::coder::Encoder::new(
                audiopus::SampleRate::Hz48000,
                opus_channels,
                audiopus::Application::Voip,
            ).expect("Erreur création encodeur Opus")
        };

        let mic_stream_opt = match mic_supported_config.sample_format() {
            cpal::SampleFormat::F32 => {
                let mut encoder = create_encoder();
                let mut mic_buffer = Vec::new();
                mic_device.build_input_stream(
                    mic_config.clone(),
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        for frame in data.chunks(mic_channels) {
                            let sum: f32 = frame.iter().sum();
                            let mono_sample = sum.clamp(-1.0, 1.0);
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
            }
            cpal::SampleFormat::I16 => {
                let mut encoder = create_encoder();
                let mut mic_buffer = Vec::new();
                mic_device.build_input_stream(
                    mic_config.clone(),
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
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
            }
            _ => None,
        };

        let mic_stream = match mic_stream_opt {
            Some(s) => s,
            None => return,
        };

        // ==========================================
        // 2. INITIALISATION DES HAUT-PARLEURS
        // ==========================================
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
                                frame[0] = l_f32;
                                frame[1] = r_f32;
                                for s in frame.iter_mut().skip(2) { *s = 0.0; }
                            }
                        }
                    },
                    err_fn_spk,
                    None,
                ).ok()
            }
            cpal::SampleFormat::I16 => {
                let mut spk_buffer: std::collections::VecDeque<i16> = std::collections::VecDeque::new();
                spk_device.build_output_stream(
                    spk_config.clone(),
                    move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                        while let Ok(pcm_chunk) = rx_spk_i16.lock().unwrap().try_recv() {
                            spk_buffer.extend(pcm_chunk);
                        }
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
            }
            _ => None,
        };

        let _ = mic_stream.play();
        if let Some(s) = &spk_stream_opt {
            let _ = s.play();
            println!("🔈 Lecture audio sur les haut-parleurs PRÊTE !");
        }

        std::thread::park();
    });
}