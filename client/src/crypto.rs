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
use std::path::Path;

const VAULT_FILE: &str = "vault.kak";

#[derive(Serialize, Deserialize, Clone)]
pub struct VaultData {
    pub private_key: [u8; 32],
    pub pseudo: String,
    pub contacts: HashMap<String, String>, // Clé Publique -> Pseudo
}

pub fn validate_password(password: &str) -> Result<(), &'static str> {
    if password.len() < 12 {
        return Err("Le mot de passe doit contenir au moins 12 caractères.");
    }
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

pub fn vault_exists() -> bool {
    Path::new(VAULT_FILE).exists()
}

/// Écrase et sauvegarde le coffre-fort entier chiffré
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

    fs::write(VAULT_FILE, file_data).map_err(|_| "Erreur IO")?;
    Ok(())
}

pub fn unlock_vault(password: &str) -> Result<VaultData, &'static str> {
    let file_data = fs::read(VAULT_FILE).map_err(|_| "Impossible de lire le coffre")?;
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
    let decrypted = cipher.decrypt(nonce, ciphertext).map_err(|_| "Mot de passe incorrect")?;

    let vault_data: VaultData = serde_json::from_slice(&decrypted).map_err(|_| "JSON invalide")?;
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