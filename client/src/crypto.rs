use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageEntry {
    pub author: String,
    pub content: String,
    pub timestamp: u64,
    pub is_media: bool,
    pub media_key: Option<[u8; 32]>,
    pub media_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GroupData {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultData {
    pub private_key: [u8; 32],
    pub pseudo: String,
    pub contacts: HashMap<String, String>,
    pub groups: HashMap<String, GroupData>,
    pub chat_history: HashMap<String, Vec<MessageEntry>>,
}

pub fn get_vault_file(pseudo: &str) -> String {
    let safe_pseudo: String = pseudo.chars().filter(|c| c.is_alphanumeric()).collect();
    format!("vault_{}.kak", safe_pseudo.to_lowercase())
}

pub fn any_vault_exists() -> bool {
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("vault_") && name.ends_with(".kak") {
                    return true;
                }
            }
        }
    }
    false
}

pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 12 { return Err("Le mot de passe doit contenir au moins 12 caractères."); }
    let (mut u, mut l, mut d, mut s) = (false, false, false, false);
    for c in password.chars() {
        if c.is_uppercase() { u = true; }
        else if c.is_lowercase() { l = true; }
        else if c.is_numeric() { d = true; }
        else { s = true; }
    }
    if u && l && d && s { Ok(()) }
    else { Err("Il faut au moins 1 Maj, 1 Min, 1 Chiffre et 1 Caractère spécial.") }
}

fn derive_key(password: &str, salt: &SaltString) -> [u8; 32] {
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), salt).unwrap();
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash.hash.unwrap().as_bytes()[..32]);
    key
}

pub fn save_vault(password: &str, data: &VaultData) -> Result<(), &'static str> {
    validate_password(password)?;
    let salt = SaltString::generate(&mut OsRng);
    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let payload = serde_json::to_vec(data).map_err(|_| "Erreur JSON")?;
    let ciphertext = cipher.encrypt(&nonce, payload.as_ref()).map_err(|_| "Erreur de chiffrement")?;

    let salt_bytes = salt.as_str().as_bytes();
    let mut file_data = Vec::new();
    file_data.extend_from_slice(&(salt_bytes.len() as u32).to_le_bytes());
    file_data.extend_from_slice(salt_bytes);
    file_data.extend_from_slice(&nonce);
    file_data.extend_from_slice(&ciphertext);

    let vault_file = get_vault_file(&data.pseudo);
    fs::write(vault_file, file_data).map_err(|_| "Erreur IO")?;
    Ok(())
}

pub fn unlock_vault(pseudo: &str, password: &str) -> Result<VaultData, &'static str> {
    let vault_file = get_vault_file(pseudo);

    let file_data = fs::read(&vault_file).map_err(|_| "Profil introuvable ou mot de passe incorrect.")?;
    if file_data.len() < 4 { return Err("Corrompu"); }

    let mut salt_len_bytes = [0u8; 4];
    salt_len_bytes.copy_from_slice(&file_data[0..4]);
    let salt_len = u32::from_le_bytes(salt_len_bytes) as usize;
    if file_data.len() < 4 + salt_len + 12 { return Err("Corrompu"); }

    let salt_str = std::str::from_utf8(&file_data[4..4 + salt_len]).map_err(|_| "Sel invalide")?;
    let salt = SaltString::from_b64(salt_str).map_err(|_| "Sel invalide")?;
    let nonce = Nonce::from_slice(&file_data[4 + salt_len..16 + salt_len]);
    let ciphertext = &file_data[16 + salt_len..];

    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(&key.into());
    let decrypted = cipher.decrypt(nonce, ciphertext).map_err(|_| "Mot de passe incorrect.")?;

    let mut vault_data: VaultData = serde_json::from_slice(&decrypted).map_err(|_| "JSON invalide")?;

    // --- NOUVEAU : LE ROLLOUT SÉCURITÉ DE 30 JOURS ---
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let thirty_days_sec = 30 * 24 * 60 * 60;
    let mut modified = false;

    for (_, history) in vault_data.chat_history.iter_mut() {
        let original_len = history.len();
        history.retain(|msg| {
            // Si le message a plus de 30 jours (ou si le calcul sature dans de rares cas d'horloge)
            if now.saturating_sub(msg.timestamp) > thirty_days_sec {
                // Si c'est un média, on écrase physiquement le fichier chiffré sur le disque
                if msg.is_media {
                    if let Some(path) = &msg.media_path {
                        let _ = std::fs::remove_file(path);
                    }
                }
                false // On supprime l'entrée de la mémoire
            } else {
                true // On garde
            }
        });
        if history.len() != original_len { modified = true; }
    }

    // On sauvegarde silencieusement la version nettoyée dans le coffre
    if modified {
        let _ = save_vault(password, &vault_data);
    }

    Ok(vault_data)
}

pub fn generate_secure_secret() -> [u8; 32] {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

pub fn derive_public_id(secret: &[u8]) -> String {
    let mut hex = String::new();
    for byte in secret.iter().take(16) { write!(&mut hex, "{:02x}", byte).unwrap(); }
    hex
}

pub fn encrypt_and_save_media(pseudo: &str, file_name: &str, raw_data: &[u8]) -> Result<([u8; 32], String), &'static str> {
    let key = generate_secure_secret();
    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, raw_data).map_err(|_| "Erreur de chiffrement du média")?;

    let media_folder = format!("media_{}", pseudo);
    let _ = fs::create_dir_all(&media_folder);
    let save_path = format!("{}/{}.enc", media_folder, file_name);

    let mut file_data = Vec::new();
    file_data.extend_from_slice(&nonce);
    file_data.extend_from_slice(&ciphertext);

    fs::write(&save_path, file_data).map_err(|_| "Impossible de sauvegarder le fichier chiffré")?;

    Ok((key, save_path))
}

pub fn decrypt_media(path: &str, key: &[u8; 32]) -> Result<Vec<u8>, &'static str> {
    let file_data = fs::read(path).map_err(|_| "Impossible de lire le fichier chiffré")?;
    if file_data.len() < 12 { return Err("Fichier corrompu"); }

    let nonce = Nonce::from_slice(&file_data[0..12]);
    let ciphertext = &file_data[12..];

    let cipher = ChaCha20Poly1305::new(key.into());
    cipher.decrypt(nonce, ciphertext).map_err(|_| "Clé de déchiffrement invalide")
}