use kira::Tween;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings};
use kira::{AudioManager, AudioManagerSettings};
use rand::Rng;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex};

fn amp_to_db(amp: f32) -> f32 {
    if amp <= 0.01 {
        -60.0
    } else {
        20.0 * amp.log10()
    }
}

lazy_static::lazy_static! {
    pub static ref SOUND_MANAGER: Arc<Mutex<SoundManager>> = Arc::new(Mutex::new(SoundManager::new()));
}

const W_MAIN_THEME: &[u8] = include_bytes!("../assets/audio/Main_Theme/AmIDreaming.wav");
const W_MSG_RX: &[u8] = include_bytes!("../assets/audio/messages/message_received.wav");
const W_MSG_TX: &[u8] = include_bytes!("../assets/audio/messages/message_sent.wav");
const W_MIC_OFF: &[u8] = include_bytes!("../assets/audio/microphone/MIC_OFF.wav");
const W_MIC_ON: &[u8] = include_bytes!("../assets/audio/microphone/MIC_ON.wav");
const W_ERR_1: &[u8] = include_bytes!("../assets/audio/errors/Error_1.wav");
const W_ERR_2: &[u8] = include_bytes!("../assets/audio/errors/Error_2.wav");
const W_BTN_1: &[u8] = include_bytes!("../assets/audio/red_buttons/button_1.wav");
const W_BTN_2: &[u8] = include_bytes!("../assets/audio/red_buttons/button_2.wav");
const W_CALL_IN: &[u8] = include_bytes!("../assets/audio/call/CALL_INGOING.wav");
const W_CALL_OUT: &[u8] = include_bytes!("../assets/audio/call/CALL_OUTGOING.wav");

pub struct SoundManager {
    manager: Option<AudioManager>,
    app_volume: f32,
    main_theme_volume: f32,
    main_theme_handle: Option<StaticSoundHandle>,
    looping_handles: HashMap<String, StaticSoundHandle>,
    cached_sounds: HashMap<&'static str, StaticSoundData>,
}

impl SoundManager {
    pub fn new() -> Self {
        let manager = match AudioManager::new(AudioManagerSettings::default()) {
            Ok(m) => Some(m),
            Err(e) => {
                println!("Audio manager ERROR: {:?}", e);
                None
            }
        };
        Self {
            manager,
            app_volume: 0.5,
            main_theme_volume: 0.1,
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
        self.app_volume
    }

    pub fn set_main_theme_volume(&mut self, volume: f32) {
        self.main_theme_volume = volume.clamp(0.0, 1.0);
        if let Some(handle) = &mut self.main_theme_handle {
            let dest_db = amp_to_db(self.main_theme_volume);
            let _ = handle.set_volume(dest_db, Tween::default());
        }
    }

    pub fn get_main_theme_volume(&self) -> f32 {
        self.main_theme_volume
    }

    pub fn play_main_theme(&mut self) {
        if self.main_theme_handle.is_none() {
            if let Some(manager) = &mut self.manager {
                let settings = StaticSoundSettings::new()
                    .volume(amp_to_db(self.main_theme_volume))
                    .loop_region(..);
                if let Ok(sound_data) = StaticSoundData::from_cursor(Cursor::new(W_MAIN_THEME)) {
                    if let Ok(handle) = manager.play(sound_data.with_settings(settings)) {
                        self.main_theme_handle = Some(handle);
                    }
                }
            }
        }
    }

    pub fn stop_main_theme(&mut self) {
        if let Some(mut handle) = self.main_theme_handle.take() {
            let _ = handle.stop(Tween::default());
        }
    }

    fn play_sound_once(&mut self, key: &'static str, bytes: &[u8], use_app_volume: bool) {
        if let Some(manager) = &mut self.manager {
            let vol = if use_app_volume { self.app_volume } else { 1.0 };
            let settings = StaticSoundSettings::new().volume(amp_to_db(vol));

            if !self.cached_sounds.contains_key(key) {
                if let Ok(sd) = StaticSoundData::from_cursor(Cursor::new(bytes)) {
                    self.cached_sounds.insert(key, sd);
                }
            }

            if let Some(sound_data) = self.cached_sounds.get(key) {
                let _ = manager.play(sound_data.clone().with_settings(settings));
            }
        }
    }

    pub fn play_message_received(&mut self) {
        self.play_sound_once("msg_rx", W_MSG_RX, true);
    }
    pub fn play_message_sent(&mut self) {
        self.play_sound_once("msg_tx", W_MSG_TX, true);
    }
    pub fn play_mic_muted(&mut self) {
        self.play_sound_once("mic_off", W_MIC_OFF, true);
    }
    pub fn play_mic_unmuted(&mut self) {
        self.play_sound_once("mic_on", W_MIC_ON, true);
    }
    pub fn play_error(&mut self) {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.5) {
            self.play_sound_once("err1", W_ERR_1, true);
        } else {
            self.play_sound_once("err2", W_ERR_2, true);
        }
    }
    pub fn play_red_button(&mut self) {
        let mut rng = rand::thread_rng();
        if rng.gen_bool(0.5) {
            self.play_sound_once("btn1", W_BTN_1, true);
        } else {
            self.play_sound_once("btn2", W_BTN_2, true);
        }
    }
    pub fn play_call_connected(&mut self) {}
    pub fn play_call_disconnected(&mut self) {}

    pub fn start_incoming_call(&mut self) {
        if self.looping_handles.contains_key("incoming_call") {
            return;
        }
        if !self.cached_sounds.contains_key("call_in") {
            if let Ok(sd) = StaticSoundData::from_cursor(Cursor::new(W_CALL_IN)) {
                self.cached_sounds.insert("call_in", sd);
            }
        }
        if let Some(sound_data) = self.cached_sounds.get("call_in") {
            if let Some(manager) = &mut self.manager {
                let settings = StaticSoundSettings::new()
                    .volume(amp_to_db(self.app_volume))
                    .loop_region(..);
                if let Ok(handle) = manager.play(sound_data.clone().with_settings(settings)) {
                    self.looping_handles
                        .insert("incoming_call".to_string(), handle);
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
        if !self.cached_sounds.contains_key("call_out") {
            if let Ok(sd) = StaticSoundData::from_cursor(Cursor::new(W_CALL_OUT)) {
                self.cached_sounds.insert("call_out", sd);
            }
        }
        if let Some(sound_data) = self.cached_sounds.get("call_out") {
            if let Some(manager) = &mut self.manager {
                let settings = StaticSoundSettings::new()
                    .volume(amp_to_db(self.app_volume))
                    .loop_region(..);
                if let Ok(handle) = manager.play(sound_data.clone().with_settings(settings)) {
                    self.looping_handles
                        .insert("outgoing_call".to_string(), handle);
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
