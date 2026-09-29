use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use kira::{AudioManager, AudioManagerSettings};
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings, StaticSoundHandle};
use kira::Tween;
use rand::Rng;

fn amp_to_db(amp: f32) -> f32 {
    if amp <= 0.01 {
        -60.0 // silence
    } else {
        20.0 * amp.log10()
    }
}

lazy_static::lazy_static! {
    pub static ref SOUND_MANAGER: Arc<Mutex<SoundManager>> = Arc::new(Mutex::new(SoundManager::new()));
}

pub struct SoundManager {
    manager: Option<AudioManager>,
    app_volume: f32,
    main_theme_volume: f32,
    main_theme_handle: Option<StaticSoundHandle>,
    looping_handles: HashMap<String, StaticSoundHandle>,
    cached_sounds: HashMap<String, StaticSoundData>,
}

impl SoundManager {
    pub fn new() -> Self {
        let manager = match AudioManager::new(AudioManagerSettings::default()) { Ok(m) => Some(m), Err(e) => { println!("Audio manager ERROR: {:?}", e); None } };
        Self {
            manager,
            app_volume: 0.5,
            main_theme_volume: 1.0,
            main_theme_handle: None,
            looping_handles: HashMap::new(),
            cached_sounds: HashMap::new(),
        }
    }

    pub fn set_app_volume(&mut self, volume: f32) {
        self.app_volume = volume.clamp(0.0, 1.0);
        let dest_db = amp_to_db(self.app_volume);
        for handle in self.looping_handles.values_mut() {
            let _ = handle.set_volume(dest_db, Tween::default());
        }
    }

    pub fn get_app_volume(&self) -> f32 {
        self.app_volume as f32
    }

    pub fn set_main_theme_volume(&mut self, volume: f32) {
        self.main_theme_volume = volume.clamp(0.0, 1.0);
        let dest_db = amp_to_db(self.main_theme_volume);
        if let Some(handle) = &mut self.main_theme_handle {
            let _ = handle.set_volume(dest_db, Tween::default());
        }
    }

    pub fn get_main_theme_volume(&self) -> f32 {
        self.main_theme_volume
    }

    pub fn play_main_theme(&mut self) {
        if self.main_theme_handle.is_none() {
            if let Some(manager) = &mut self.manager {
                let settings = StaticSoundSettings::new().volume(amp_to_db(self.main_theme_volume)).loop_region(..);
                match StaticSoundData::from_file(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/Main_Theme/AmIDreaming.wav")) {
                    Ok(sound_data) => {
                        match manager.play(sound_data.with_settings(settings)) {
                            Ok(handle) => {
                                self.main_theme_handle = Some(handle);
                            }
                            Err(e) => println!("Play Main Theme error: {:?}", e),
                        }
                    }
                    Err(e) => println!("Load Main Theme error: {:?}", e),
                }
            }
        }
    }

    pub fn stop_main_theme(&mut self) {
        if let Some(mut handle) = self.main_theme_handle.take() {
            let _ = handle.stop(Tween::default());
        }
    }

    fn play_sound_once(&mut self, path: impl AsRef<std::path::Path>, use_app_volume: bool) {
        if let Some(manager) = &mut self.manager {
            let vol = if use_app_volume { self.app_volume } else { 1.0 };
            let settings = StaticSoundSettings::new().volume(amp_to_db(vol));
            
            let path_str = path.as_ref().to_string_lossy().to_string();
            if !self.cached_sounds.contains_key(&path_str) {
                if let Ok(sd) = StaticSoundData::from_file(&path) {
                    self.cached_sounds.insert(path_str.clone(), sd);
                }
            }
            
            if let Some(sound_data) = self.cached_sounds.get(&path_str) {
                let _ = manager.play(sound_data.clone().with_settings(settings));
            }
        }
    }

    pub fn play_message_received(&mut self) {
        self.play_sound_once(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/messages/message_received.wav"), true);
    }

    pub fn play_message_sent(&mut self) {
        self.play_sound_once(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/messages/message_sent.wav"), true);
    }

    pub fn play_mic_muted(&mut self) {
        self.play_sound_once(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/microphone/MIC_OFF.wav"), true);
    }

    pub fn play_mic_unmuted(&mut self) {
        self.play_sound_once(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/microphone/MIC_ON.wav"), true);
    }

    pub fn play_error(&mut self) {
        let mut rng = rand::thread_rng();
        let paths = [std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/errors/Error_1.wav"), std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/errors/Error_2.wav")];
        let choice = paths[rng.gen_range(0..paths.len())].clone();
        self.play_sound_once(choice, true);
    }

    pub fn play_red_button(&mut self) {
        let mut rng = rand::thread_rng();
        let paths = [std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/red_buttons/button_1.wav"), std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/red_buttons/button_2.wav")];
        let choice = paths[rng.gen_range(0..paths.len())].clone();
        self.play_sound_once(choice, true);
    }

    pub fn play_call_connected(&mut self) {
    }

    pub fn play_call_disconnected(&mut self) {
    }

    pub fn start_incoming_call(&mut self) {
        if self.looping_handles.contains_key("incoming_call") {
            return;
        }
        if let Some(manager) = &mut self.manager {
            let settings = StaticSoundSettings::new().volume(amp_to_db(self.app_volume)).loop_region(..);
            
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/call/CALL_INGOING.wav");
            let path_str = path.to_string_lossy().to_string();
            
            if !self.cached_sounds.contains_key(&path_str) {
                if let Ok(sd) = StaticSoundData::from_file(&path) {
                    self.cached_sounds.insert(path_str.clone(), sd);
                }
            }
            if let Some(sound_data) = self.cached_sounds.get(&path_str) {
                if let Ok(handle) = manager.play(sound_data.clone().with_settings(settings)) {
                    self.looping_handles.insert("incoming_call".to_string(), handle);
                }
            }
        }
    }

    pub fn stop_incoming_call(&mut self) {
        if let Some(mut handle) = self.looping_handles.remove("incoming_call") {
            let _ = handle.stop(Tween::default());
        }
    }

    pub fn start_outgoing_call(&mut self) {
        if self.looping_handles.contains_key("outgoing_call") {
            return;
        }
        if let Some(manager) = &mut self.manager {
            let settings = StaticSoundSettings::new().volume(amp_to_db(self.app_volume)).loop_region(..);
            
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/audio/call/CALL_OUTGOING.wav");
            let path_str = path.to_string_lossy().to_string();
            
            if !self.cached_sounds.contains_key(&path_str) {
                if let Ok(sd) = StaticSoundData::from_file(&path) {
                    self.cached_sounds.insert(path_str.clone(), sd);
                }
            }
            if let Some(sound_data) = self.cached_sounds.get(&path_str) {
                if let Ok(handle) = manager.play(sound_data.clone().with_settings(settings)) {
                    self.looping_handles.insert("outgoing_call".to_string(), handle);
                }
            }
        }
    }

    pub fn stop_outgoing_call(&mut self) {
        if let Some(mut handle) = self.looping_handles.remove("outgoing_call") {
            let _ = handle.stop(Tween::default());
        }
    }
}
