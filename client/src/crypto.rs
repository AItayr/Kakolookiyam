use ring::signature::{Ed25519KeyPair, KeyPair};
use ring::rand::SystemRandom;
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub struct Identity {
    pub public_key_b64: String,
}

pub fn generate_identity() -> Identity {
    println!("🔐 Génération de l'identité cryptographique en RAM (Ed25519)...");

    // Générateur de nombres aléatoires sécurisé par le système (OS)
    let rng = SystemRandom::new();

    // Création de la paire de clés (Privée / Publique)
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
        .expect("Erreur lors de la génération de la clé");

    let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
        .expect("Erreur lors de la lecture de la clé");

    // Extraction de la clé publique brute (octets)
    let pub_key_bytes = key_pair.public_key().as_ref();

    // Encodage en Base64 pour l'afficher proprement dans l'interface
    let public_key_b64 = STANDARD.encode(pub_key_bytes);

    Identity {
        public_key_b64,
    }
}