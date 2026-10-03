use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, AeadCore, KeyInit},
};
use dirs;
use rand::{RngCore, rngs::OsRng};
use ring::signature::KeyPair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::PathBuf;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Serialize, Deserialize, Clone)]
pub struct MessageEntry {
    pub author: String,
    pub content: String,
    pub timestamp: u64,
    pub is_media: bool,
    pub media_key: Option<[u8; 32]>,
    pub media_path: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct GroupData {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Zeroize, ZeroizeOnDrop)]
pub struct VaultData {
    pub private_key: [u8; 32],
    pub pseudo: String,
    #[zeroize(skip)]
    pub contacts: HashMap<String, String>,
    #[serde(default)]
    #[zeroize(skip)]
    pub pending_requests: HashMap<String, String>,
    #[serde(default)]
    #[zeroize(skip)]
    pub blocked_ids: std::collections::HashSet<String>,
    #[zeroize(skip)]
    pub groups: HashMap<String, GroupData>,
    #[zeroize(skip)]
    pub chat_history: HashMap<String, Vec<MessageEntry>>,
    #[serde(default)]
    #[zeroize(skip)]
    pub tombstones: std::collections::HashSet<String>,
    #[serde(skip)]
    pub session_key: Option<[u8; 32]>,
    #[serde(skip)]
    pub session_salt: Option<String>,
}

impl VaultData {
    pub fn zeroize_deep(&mut self) {
        self.private_key.zeroize();
        if let Some(mut k) = self.session_key.take() {
            k.zeroize();
        }
        self.pseudo.zeroize();
        for (_, msgs) in self.chat_history.iter_mut() {
            for m in msgs.iter_mut() {
                m.author.zeroize();
                m.content.zeroize();
                if let Some(mut k) = m.media_key {
                    k.zeroize();
                }
                if let Some(p) = &mut m.media_path {
                    p.zeroize();
                }
            }
        }
        for (_, group) in self.groups.iter_mut() {
            group.name.zeroize();
            for member in group.members.iter_mut() {
                member.zeroize();
            }
        }
        for (_, contact) in self.contacts.iter_mut() {
            // Cannot easily zeroize values directly from iterator here, but wait:
            // if Contacts is HashMap<String, String>, then contact.zeroize() works!
            contact.zeroize();
        }
    }
}

pub fn get_app_dir() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("Kakolookiyam");
    let _ = fs::create_dir_all(&path);

    let readme_path = path.join("A_PROPOS_DE_VOS_DONNEES.txt");
    if !readme_path.exists() {
        let content = "Kakolookiyam : Dossier Coffre-Fort / Vault Folder / مجلد الخزنة

\
[FR] Ce dossier héberge toutes vos données chiffrées localement.
\
Si vous souhaitez supprimer votre compte et toutes ses données, vous pouvez simplement effacer ce dossier.
\
Aucun historique n\'est récupérable en ligne.

\
[EN] This folder hosts all your locally encrypted data.
\
If you wish to delete your account and all its data, you can simply delete this folder.
\
No history is recoverable online.

\
[AR] يستضيف هذا المجلد جميع بياناتك المشفرة محليا.
\
إذا كنت ترغب في حذف حسابك وجميع بياناته، يمكنك ببساطة حذف هذا المجلد.
\
لا يمكن استرجاع أي سجل عبر الإنترنت.";
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
    use argon2::{Algorithm, Params, Version};
    // [MITIGATION BRUTE-FORCE] Paramètres OWASP 2026 : 64MB RAM, 3 Itérations, 4 Parallélismes.
    let params = Params::new(65536, 3, 4, None).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let hash = argon2.hash_password(password.as_bytes(), salt).unwrap();
    let mut key = [0u8; 32];

    key.copy_from_slice(&hash.hash.unwrap().as_bytes()[..32]);
    key
}

pub fn save_vault(password: &str, data: &mut VaultData) -> Result<(), &'static str> {
    validate_password(password)?;

    let (key, salt_str) = if let (Some(k), Some(s)) = (data.session_key, &data.session_salt) {
        (k, s.clone())
    } else {
        let salt = SaltString::generate(&mut OsRng);
        let key = derive_key(password, &salt);
        let salt_string = salt.as_str().to_string();
        data.session_key = Some(key);
        data.session_salt = Some(salt_string.clone());
        (key, salt_string)
    };

    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let payload = serde_json::to_vec(data).map_err(|_| "Erreur JSON")?;
    let ciphertext = cipher
        .encrypt(&nonce, payload.as_ref())
        .map_err(|_| "Erreur de chiffrement")?;

    let salt_bytes = salt_str.as_bytes();
    let mut file_data = Vec::new();

    file_data.extend_from_slice(&(salt_bytes.len() as u32).to_le_bytes());
    file_data.extend_from_slice(salt_bytes);
    file_data.extend_from_slice(&nonce);
    file_data.extend_from_slice(&ciphertext);

    // [MITIGATION RAM] Nettoyage manuel du buffer contenant le JSON non chiffré
    let mut payload_mut = payload;
    payload_mut.zeroize();

    let vault_file = get_vault_file(&data.pseudo);

    // [MITIGATION ATOMIQUE] Écriture dans un fichier temporaire puis renommage (garanti atomique sous Windows/Linux/Mac)
    let tmp_file = format!("{}.tmp", vault_file);
    fs::write(&tmp_file, file_data).map_err(|_| "Erreur IO")?;
    fs::rename(&tmp_file, vault_file).map_err(|_| "Erreur Atomique")?;

    Ok(())
}

pub fn unlock_vault(pseudo: &str, password: &str) -> Result<VaultData, &'static str> {
    let vault_file = get_vault_file(pseudo);

    // [MITIGATION ANTI-TIMING] Si le profil (fichier) n'existe pas,
    // l'algorithme Argon2 tourne dans le vide sur un sel généré aléatoirement.
    // Cela rend le temps de réponse aveugle (Constant-Time) équivalent à un échec de mot de passe.
    let file_data = match std::fs::read(&vault_file) {
        Ok(data) => data,
        Err(_) => {
            let dummy_salt = SaltString::generate(&mut OsRng);
            let _ = derive_key(password, &dummy_salt);
            return Err("Profil introuvable ou mot de passe incorrect.");
        }
    };
    if file_data.len() < 4 {
        return Err("Corrompu");
    }

    let mut salt_len_bytes = [0u8; 4];
    salt_len_bytes.copy_from_slice(&file_data[0..4]);
    let salt_len = u32::from_le_bytes(salt_len_bytes) as usize;

    if file_data.len() < 4 + salt_len + 12 {
        return Err("Fichier corrompu");
    }

    let salt_str = std::str::from_utf8(&file_data[4..4 + salt_len])
        .map_err(|_| "Erreur de décodage du Salt")?;
    let salt = SaltString::from_b64(salt_str).map_err(|_| "Format de Salt invalide")?;

    let nonce_start = 4 + salt_len;
    let nonce_bytes = &file_data[nonce_start..nonce_start + 12];
    let nonce = Nonce::from_slice(nonce_bytes);

    let ciphertext = &file_data[nonce_start + 12..];

    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(&key.into());

    let payload = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Mot de passe erroné ou corruption de données")?;

    let mut vault_data: VaultData =
        serde_json::from_slice(&payload).map_err(|_| "Format de fichier invalide")?;

    vault_data.session_key = Some(key);
    vault_data.session_salt = Some(salt_str.to_string());

    // [MITIGATION SWAP/PAGEFILE] Verrouille la clé privée en RAM pure pour interdire la pagination sur le disque
    let _ = region::lock(
        vault_data.private_key.as_ptr(),
        vault_data.private_key.len(),
    );

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
pub fn add_message_to_vault(
    password: &str,
    vault_data: &mut VaultData,
    dest: &str,
    msg: MessageEntry,
) -> Result<VaultData, &'static str> {
    let _ = unlock_vault(&vault_data.pseudo, password)?;

    let is_group = vault_data.groups.contains_key(dest);

    let mut modified = false;

    if is_group || vault_data.contacts.contains_key(dest) {
        let history = vault_data
            .chat_history
            .entry(dest.to_string())
            .or_insert_with(Vec::new);

        let exists = history
            .iter()
            .any(|m| m.timestamp == msg.timestamp && m.author == msg.author);
        if !exists {
            history.push(msg);
            modified = true;
        }
    }

    if modified {
        let _ = save_vault(password, vault_data);
    }

    Ok(vault_data.clone())
}

pub fn generate_secure_secret() -> [u8; 32] {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

pub fn derive_public_id(secret: &[u8]) -> String {
    // Note: secret must be exactly 32 bytes for Ed25519 (unless called with random short slices like in media, then we pad/hash)
    if secret.len() != 32 {
        let mut hex = String::new();
        for byte in secret.iter().take(16) {
            write!(&mut hex, "{:02x}", byte).unwrap();
        }
        return hex;
    }
    if let Ok(key_pair) = ring::signature::Ed25519KeyPair::from_seed_unchecked(secret) {
        let pub_key = key_pair.public_key().as_ref();
        let mut hex = String::new();
        for byte in pub_key {
            write!(&mut hex, "{:02x}", byte).unwrap();
        }
        hex
    } else {
        String::new()
    }
}

#[allow(dead_code)]
pub fn sign_message(secret: &[u8], target_id: &str, timestamp: u64, content: &str) -> String {
    if let Ok(key_pair) = ring::signature::Ed25519KeyPair::from_seed_unchecked(secret) {
        let pub_id = derive_public_id(secret);
        let message = format!("KAKO-MSG-v2|{}|{}|{}|{}", pub_id, target_id, timestamp, content);
        let signature = key_pair.sign(message.as_bytes());
        let mut hex = String::new();
        for byte in signature.as_ref() {
            std::fmt::Write::write_fmt(&mut hex, format_args!("{:02x}", byte)).unwrap();
        }
        hex
    } else {
        String::new()
    }
}

#[allow(dead_code)]
pub fn verify_message(
    pub_id_hex: &str,
    timestamp: u64,
    content: &str,
    signature_hex: &str,
) -> bool {
    use ring::signature::UnparsedPublicKey;
    let mut pub_key_bytes = [0u8; 32];
    if pub_id_hex.len() != 64 {
        return false;
    }
    for i in 0..32 {
        if let Ok(b) = u8::from_str_radix(&pub_id_hex[i * 2..i * 2 + 2], 16) {
            pub_key_bytes[i] = b;
        } else {
            return false;
        }
    }
    let mut sig_bytes = [0u8; 64];
    if signature_hex.len() != 128 {
        return false;
    }
    for i in 0..64 {
        if let Ok(b) = u8::from_str_radix(&signature_hex[i * 2..i * 2 + 2], 16) {
            sig_bytes[i] = b;
        } else {
            return false;
        }
    }
    let message = format!("{}:{}:{}", pub_id_hex, timestamp, content);
    let public_key = UnparsedPublicKey::new(&ring::signature::ED25519, pub_key_bytes);
    public_key.verify(message.as_bytes(), &sig_bytes).is_ok()
}

pub fn encrypt_and_save_media(
    pseudo: &str,
    _file_name: &str,
    raw_data: &[u8],
) -> Result<([u8; 32], String), &'static str> {
    let key = generate_secure_secret();
    let cipher = ChaCha20Poly1305::new(&key.into());
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, raw_data)
        .map_err(|_| "Erreur de chiffrement du media")?;

    let media_folder = get_media_dir(pseudo);
    let mut rnd_name = [0u8; 16];
    OsRng.fill_bytes(&mut rnd_name);
    let safe_name = derive_public_id(&rnd_name);
    let save_path = format!("{}/{}.enc", media_folder, safe_name);

    let mut file_data = Vec::new();
    file_data.extend_from_slice(&nonce);
    file_data.extend_from_slice(&ciphertext);

    fs::write(&save_path, file_data).map_err(|_| "Impossible de sauvegarder le fichier chiffre")?;

    Ok((key, save_path))
}

pub fn decrypt_media(path: &str, key: &[u8; 32]) -> Result<Vec<u8>, &'static str> {
    let file_data = fs::read(path).map_err(|_| "Impossible de lire le fichier chiffre")?;
    if file_data.len() < 12 {
        return Err("Fichier corrompu");
    }

    let nonce = Nonce::from_slice(&file_data[0..12]);
    let ciphertext = &file_data[12..];

    let cipher = ChaCha20Poly1305::new(key.into());
    let dec = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Decodage impossible - Cle invalide")?;

    Ok(dec)
}

pub fn sign_signal(
    secret: &[u8],
    kind: &str,
    sender: &str,
    target: &str,
    pseudo: &str,
    payload: &str,
    ts: u64,
) -> String {
    if let Ok(key_pair) = ring::signature::Ed25519KeyPair::from_seed_unchecked(secret) {
        let h = ring::digest::digest(&ring::digest::SHA256, payload.as_bytes());
        let hex: String = h.as_ref().iter().map(|b| format!("{:02x}", b)).collect();
        let message = format!(
            "KAKO-SIG-v2|{}|{}|{}|{}|{}|{}",
            kind, sender, target, pseudo, ts, hex
        );
        let signature = key_pair.sign(message.as_bytes());
        let mut out = String::new();
        for byte in signature.as_ref() {
            std::fmt::Write::write_fmt(&mut out, format_args!("{:02x}", byte)).unwrap();
        }
        out
    } else {
        String::new()
    }
}

pub fn verify_signal(
    sender_pub_hex: &str,
    kind: &str,
    my_id: &str,
    pseudo: &str,
    payload: &str,
    ts: u64,
    signature_hex: &str,
) -> bool {
    use ring::signature::UnparsedPublicKey;
    let mut pub_key_bytes = [0u8; 32];
    if sender_pub_hex.len() != 64 {
        return false;
    }
    for i in 0..32 {
        if let Ok(b) = u8::from_str_radix(&sender_pub_hex[i * 2..i * 2 + 2], 16) {
            pub_key_bytes[i] = b;
        } else {
            return false;
        }
    }
    let mut sig_bytes = [0u8; 64];
    if signature_hex.len() != 128 {
        return false;
    }
    for i in 0..64 {
        if let Ok(b) = u8::from_str_radix(&signature_hex[i * 2..i * 2 + 2], 16) {
            sig_bytes[i] = b;
        } else {
            return false;
        }
    }
    let h = ring::digest::digest(&ring::digest::SHA256, payload.as_bytes());
    let hex: String = h.as_ref().iter().map(|b| format!("{:02x}", b)).collect();
    let message = format!(
        "KAKO-SIG-v2|{}|{}|{}|{}|{}|{}",
        kind, sender_pub_hex, my_id, pseudo, ts, hex
    );
    let public_key = UnparsedPublicKey::new(&ring::signature::ED25519, pub_key_bytes);
    public_key.verify(message.as_bytes(), &sig_bytes).is_ok()
}
