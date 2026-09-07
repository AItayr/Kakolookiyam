use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use nnnoiseless::DenoiseState;

// On active la réduction de bruit IA par défaut au démarrage !
static IS_ENABLED: AtomicBool = AtomicBool::new(true);

pub static DENOISER: Mutex<Option<Box<DenoiseState<'static>>>> = Mutex::new(None);

pub fn set_enabled(enabled: bool) {
    IS_ENABLED.store(enabled, Ordering::Relaxed);
    if !enabled {
        // On purge la mémoire vive (RAM) si l'utilisateur décide de l'éteindre
        *DENOISER.lock().unwrap() = None;
    }
}

pub fn is_enabled() -> bool {
    IS_ENABLED.load(Ordering::Relaxed)
}

pub fn process_chunk(frame: &mut [i16]) {
    // Si l'IA n'est pas activée, on laisse l'audio intact et on ne perd aucun temps
    if !IS_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    let mut lock = DENOISER.lock().unwrap();
    
    // Chargement "paresseux" (Lazy-loading) à la volée dès qu'on capte du son
    if lock.is_none() {
        *lock = Some(DenoiseState::new());
    }

    if let Some(denoiser) = lock.as_mut() {
        let frame_size = nnnoiseless::DenoiseState::FRAME_SIZE;
        
        let mut buffer_in = [0.0f32; 480];
        let mut buffer_out = [0.0f32; 480];
        
        for chunk in frame.chunks_mut(frame_size) {
            if chunk.len() == frame_size {
                // Conversion Int16 -> Float32 pour l'Intelligence Artificielle
                for (in_s, buf_s) in chunk.iter().zip(buffer_in.iter_mut()) {
                    *buf_s = *in_s as f32;
                }
                
                // Nettoyage de la tranche audio
                denoiser.process_frame(&mut buffer_out, &buffer_in);
                
                // On remet proprement le flux propre pour le WebRTC
                for (buf_s, out_s) in buffer_out.iter().zip(chunk.iter_mut()) {
                    *out_s = buf_s.clamp(i16::MIN as f32, i16::MAX as f32) as i16;
                }
            }
        }
    }
}
