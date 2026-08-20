use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use rand::{rngs::OsRng, RngCore}; // NOUVEAU : On utilise directement le OsRng de "rand"
use std::fmt::Write;
use std::fs;
use std::path::Path;

const VAULT_FILE: &str = "vault.kak";

/// Dérive une clé de 32 octets à partir d'un mot de passe et d'un sel
fn derive_key(password: &str, salt: &SaltString) -> [u8; 32] {
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), salt).unwrap();

    let mut key = [0u8; 32];
    let hash_bytes = hash.hash.unwrap();
    key.copy_from_slice(&hash_bytes.as_bytes()[..32]);
    key
}

/// Vérifie si un coffre-fort existe déjà sur le PC
pub fn vault_exists() -> bool {
    Path::new(VAULT_FILE).exists()
}

/// Créer le coffre-fort et chiffrer la clé secrète
pub fn create_vault(password: &str, private_key: &[u8]) -> Result<(), &'static str> {
    // 1. Générer un "sel" aléatoire avec OsRng
    let salt = SaltString::generate(&mut OsRng);

    // 2. Transformer le mot de passe humain en clé cryptographique via Argon2
    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(&key.into());

    // 3. Générer un Nonce à usage unique
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

    // 4. Chiffrer la clé secrète
    let ciphertext = cipher
        .encrypt(&nonce, private_key)
        .map_err(|_| "Erreur de chiffrement interne")?;

    // 5. Emballer les données : [Taille Sel] + [Sel] + [Nonce] + [Données Chiffrées]
    let salt_bytes = salt.as_str().as_bytes();
    let mut file_data = Vec::new();
    file_data.extend_from_slice(&(salt_bytes.len() as u32).to_le_bytes());
    file_data.extend_from_slice(salt_bytes);
    file_data.extend_from_slice(&nonce);
    file_data.extend_from_slice(&ciphertext);

    // 6. Sauvegarde sur le disque dur
    fs::write(VAULT_FILE, file_data).map_err(|_| "Erreur de sauvegarde sur le disque")?;

    Ok(())
}

/// Déverrouiller le coffre-fort
pub fn unlock_vault(password: &str) -> Result<Vec<u8>, &'static str> {
    // 1. Lire le fichier local
    let file_data = fs::read(VAULT_FILE).map_err(|_| "Impossible de lire le coffre-fort")?;
    if file_data.len() < 4 {
        return Err("Fichier corrompu");
    }

    // 2. Extraire la longueur du sel
    let mut salt_len_bytes = [0u8; 4];
    salt_len_bytes.copy_from_slice(&file_data[0..4]);
    let salt_len = u32::from_le_bytes(salt_len_bytes) as usize;
    if file_data.len() < 4 + salt_len + 12 {
        return Err("Fichier corrompu");
    }

    // 3. Extraire le sel
    let salt_str =
        std::str::from_utf8(&file_data[4..4 + salt_len]).map_err(|_| "Sel invalide")?;
    let salt = SaltString::from_b64(salt_str).map_err(|_| "Sel invalide")?;

    // 4. Extraire le Nonce
    let nonce_start = 4 + salt_len;
    let nonce_end = nonce_start + 12;
    let nonce = Nonce::from_slice(&file_data[nonce_start..nonce_end]);

    // 5. Extraire le contenu chiffré
    let ciphertext = &file_data[nonce_end..];

    // 6. Déchiffrement ChaCha20-Poly1305
    let key = derive_key(password, &salt);
    let cipher = ChaCha20Poly1305::new(&key.into());

    let decrypted_data = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Mot de passe incorrect")?;

    Ok(decrypted_data)
}

/// Génère une clé privée de 32 octets de manière cryptographiquement sécurisée (CSPRNG OsRng)
pub fn generate_secure_secret() -> [u8; 32] {
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    secret
}

/// Dérive un identifiant public (ID) à partir de la clé secrète
pub fn derive_public_id(secret: &[u8]) -> String {
    let mut hex = String::new();
    for byte in secret.iter().take(16) {
        write!(&mut hex, "{:02x}", byte).unwrap();
    }
    hex
}