# SecureP2P-Voice

A lightweight, high-performance, and ultra-secure cross-platform peer-to-peer (P2P) voice communication application inspired by classic early-era Skype. Built completely natively without web technologies to ensure maximum performance and absolute user data privacy.

---

## 🔒 Security & Architecture Philosophy

This project adopts a strict **zero-trust, zero-trace** design:

* **No Network Persistence:** No persistent databases, user logs, or historical data are ever stored on the network.
* **RAM-Only Processing:** Sensitive data, encryption keys, and audio chunks reside strictly in temporary memory (RAM) and are wiped out immediately upon closing the application.
* **End-to-End Encryption (E2EE):** All direct P2P streams are strongly encrypted using ciphers like **AES-256** or **ChaCha20**, with keys dynamically generated on-the-fly per session.
* **Blind Signal Server:** A minimal signaling server handles initial connection matching anonymously without recording IP addresses or metadata.

---

## 🛠️ Technology Stack

* **Language:** **Rust** — Chosen for its extreme performance, zero garbage collector, and absolute memory safety.
* **User Interface (GUI):** **Iced** or **Slint** — Provides a streamlined, lightweight, and efficient native interface across platforms.
* **Networking & P2P:** **Tokio** (for high-performance asynchronous networking) and **webrtc-rs** (for native P2P audio/video handling and NAT traversal).
* **Cryptography:** **ring** or **sodiumoxide** for bulletproof memory-level encryption.

---

## 🚀 Supported Platforms

Compiled natively into lightweight binaries for:

* **Windows**
* **Linux**
* **macOS**