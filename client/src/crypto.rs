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
use std::path::PathBuf;
use dirs;

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

pub fn get_app_dir() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("Kakolookiyam");
    let _ = fs::create_dir_all(&path);
    
    let readme_path = path.join("A_PROPOS_DE_VOS_DONNEES.txt");
    if !readme_path.exists() {
        let content = "Kakolookiyam : Dossier Coffre-Fort / Vault Folder / مجلد الخزنة\r\n\r\n\
[FR] Ce dossier héberge toutes vos données chiffrées localement.\r\n\
Si vous souhaitez supprimer votre compte et toutes ses données, vous pouvez simplement effacer ce dossier.\r\n\
Aucun historique n'est récupérable en ligne.\r\n\r\n\
[EN] This folder hosts all your locally encrypted data.\r\n\
If you wish to delete your account and all its data, you can simply delete this folder.\r\n\
No history is recoverable online.\r\n\r\n\
[AR] يستضيف هذا المجلد جميع بياناتك المشفرة محليًا.\r\n\
إذا كنت ترغب في حذف حسابك وجميع بياناته، يمكنك ببساطة حذف هذا المجلد.\r\n\
لا يمكن استرداد أي سجل عبر الإنترنت.";
        let _ = fs::write(readme_path, content);
    }
    
    path
}

pub fn get_vault_file(pseudo: &str) -> String {
    let safe_pseudo: String = pseudo.chars().filter(|c| c.is_alphanumeric()).collect();
    let file = format!("vault_{}.kak", safe_pseudo.to_lowercase());
    get_app_dir().join(file).to_string_lossy().into_owned()
}

pub fn get_media_dir(pseudo: &str) -> String {
    let safe_pseudo: String = pseudo.chars().filter(|c| c.is_alphanumeric()).collect();
    let media = format!("media_{}", safe_pseudo.to_lowercase());
    let path = get_app_dir().join(media);
    let _ = fs::create_dir_all(&path);
    path.to_string_lossy().into_owned()
}

pub fn any_vault_exists() -> bool {
    let path = get_app_dir();
    if let Ok(entries) = std::fs::read_dir(path) {
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
    if password.len() < 12 {
        return Err("Le mot de passe doit contenir au moins 12 caractères.");
    }

    let (mut u, mut l, mut d, mut s) = (false, false, false, false);

    for c in password.chars() {
        if c.is_uppercase() {
            u = true;
        } else if c.is_lowercase() {
            l = true;
        } else if c.is_numeric() {
            d = true;
        } else {
            s = true;
        }
    }

    if u && l && d && s {
        Ok(())
    } else {
        Err("Il faut au moins 1 Maj, 1 Min, 1 Chiffre et 1 Caractère spécial.")
    }
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
    if file_data.len() < 4 {
        return Err("Corrompu");
    }

    let mut salt_len_bytes = [0u8; 4];
    salt_len_bytes.copy_from_slice(&file_data[0..4]);
    let salt_len = u32::from_le_bytes(salt_len_bytes) as usize;

    if file_data.len() < 4 + salt_len + 12 {
        return Err("Fichier corrompu");
    }

    let salt_str = std::str::from_utf8(&file_data[4..4 + salt_len]).map_err(|_| "Erreur de décodage du Salt")?;
    let salt = SaltString::from_b64(salt_str).map_err(|_| "Format de Salt invalide")?;

    let nonce_start = 4 + salt_len;
    let nonce_bytes = &file_data[nonce_start..nonce_start + 12];
    let nonce = Nonce::from_slice(nonce_bytes);

    let ciphertext = &file_data[nonce_start + 12..];

    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(&key.into());

    let payload = cipher.decrypt(nonce, ciphertext).map_err(|_| "Mot de passe erroné ou corruption de données")?;

    let vault_data: VaultData = serde_json::from_slice(&payload).map_err(|_| "Format de fichier invalide")?;

    Ok(vault_data)
}

#[allow(dead_code)]
pub fn delete_vault(pseudo: &str, password: &str) -> std::result::Result<(), String> {
    if let Ok(_vault_data) = unlock_vault(pseudo, password) {
        let vault_file = get_vault_file(pseudo);
        let _ = std::fs::remove_file(vault_file);
        
        let media_folder = get_media_dir(pseudo);
        let _ = std::fs::remove_dir_all(media_folder);
        
        Ok(())
    } else {
        Err("Mot de passe incorrect".to_string())
    }
}

#[allow(dead_code)]
pub fn add_message_to_vault(password: &str, vault_data: &mut VaultData, dest: &str, msg: MessageEntry) -> Result<VaultData, &'static str> {
    let _ = unlock_vault(&vault_data.pseudo, password)?;

    let is_group = vault_data.groups.contains_key(dest);
    
    let mut modified = false;

    if is_group || vault_data.contacts.contains_key(dest) {
        let history = vault_data.chat_history.entry(dest.to_string()).or_insert_with(Vec::new);
        
        let exists = history.iter().any(|m| m.timestamp == msg.timestamp && m.author == msg.author);
        if !exists {
            history.push(msg);
            modified = true;
        }
    }

    if modified {
        let _ = save_vault(password, &vault_data);
    }

    Ok(vault_data.clone())
}

pub fn generate_secure_secret() -> [u8; 32] {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

pub fn derive_public_id(secret: &[u8]) -> String {
    let mut hex = String::new();
    for byte in secret.iter().take(16) {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }
    hex
}

pub fn encrypt_and_save_media(pseudo: &str, file_name: &str, raw_data: &[u8]) -> Result<([u8; 32], String), &'static str> {
    let key = generate_secure_secret();
    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let ciphertext = cipher.encrypt(&nonce, raw_data).map_err(|_| "Erreur de chiffrement du média")?;

    let media_folder = get_media_dir(pseudo);
    let save_path = format!("{}/{}.enc", media_folder, file_name); // It's okay because get_media_dir returns absolute

    let mut file_data = Vec::new();
    file_data.extend_from_slice(&nonce);
    file_data.extend_from_slice(&ciphertext);

    fs::write(&save_path, file_data).map_err(|_| "Impossible de sauvegarder le fichier chiffré")?;

    Ok((key, save_path))
}

pub fn decrypt_media(path: &str, key: &[u8; 32]) -> Result<Vec<u8>, &'static str> {
    let file_data = fs::read(path).map_err(|_| "Impossible de lire le fichier chiffré")?;
    if file_data.len() < 12 {
        return Err("Fichier corrompu");
    }

    let nonce = Nonce::from_slice(&file_data[0..12]);
    let ciphertext = &file_data[12..];

    let cipher = ChaCha20Poly1305::new(key.into());
    let dec = cipher.decrypt(nonce, ciphertext).map_err(|_| "Décodage impossible - Clé invalide")?;
    
    Ok(dec)
}
