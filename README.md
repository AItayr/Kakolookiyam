# Kakolookiyam 🛡️

*Connected together. Invisible to the rest.*

A lightweight, high-performance, and ultra-secure cross-platform peer-to-peer (P2P "Full Mesh") communication application. Built natively in Rust without web technologies to ensure maximum performance and absolute user data privacy.

## 🔒 Security & Architecture Philosophy (Zero-Knowledge)

This project adopts a strict **zero-trust, zero-trace** design:

* **No Network Persistence:** The blind signaling server handles initial connection matching anonymously without recording IP addresses or metadata.


* **Local Vault:** Your identity, contacts, and chat history are encrypted locally using **ChaCha20Poly1305** and **Argon2**.


* **RAM-Only Processing:** Sensitive data and media previews reside strictly in temporary memory (RAM) and are wiped out immediately upon locking the vault.


* **End-to-End Encryption (E2EE):** All direct P2P streams and heavy file transfers (chunked with ACK) are strictly encrypted.



## 🛠️ Technology Stack

* **Language:** **Rust** — Chosen for extreme performance, zero garbage collector, and absolute memory safety.


* **User Interface (GUI):** **Iced** — Provides a streamlined, native interface across Windows, macOS, and Linux.


* **Networking & P2P:** **Tokio** (async) and **WebRTC** (native P2P mesh network, audio, and NAT traversal).



## ⚖️ License & Independence

Kakolookiyam is proudly open-source and distributed under the **GNU Affero General Public License v3.0 (AGPLv3)**. This guarantees that the network architecture remains transparent, auditable, and fiercely protected against proprietary corporate appropriation.

## 💖 Support the Project

Kakolookiyam is built for the people, not for data brokers. If this tool guarantees your digital freedom, consider supporting its independent development:
👉 **[Sponsor Altayr on GitHub](https://github.com/sponsors/AItayr)**

---

# Kakolookiyam 🛡️

*Connectés entre vous. Invisibles pour le reste.*

Une application de communication pair-à-pair (P2P "Full Mesh") multiplateforme, légère et ultra-sécurisée. Conçue entièrement en natif avec Rust, sans technologies web, pour garantir des performances maximales et une confidentialité absolue.

## 🔒 Philosophie de Sécurité (Zero-Knowledge)

Ce projet adopte une architecture stricte **zéro-confiance, zéro-trace** :

* **Aucune persistance réseau :** Le serveur de signalisation aveugle gère la mise en relation initiale de manière anonyme, sans enregistrer d'adresses IP ni de métadonnées.


* **Coffre-fort local :** Votre identité, vos contacts et votre historique sont chiffrés localement via **ChaCha20Poly1305** et **Argon2**.


* **Traitement exclusif en RAM :** Les données sensibles et les aperçus de médias résident uniquement en mémoire vive et sont purgés immédiatement au verrouillage.


* **Chiffrement de bout en bout (E2EE) :** Tous les flux P2P directs et les transferts de fichiers lourds sont strictement chiffrés.



## 🛠️ Stack Technique

* **Langage :** **Rust** — Choisi pour ses performances extrêmes, l'absence de garbage collector et sa sécurité mémoire absolue.


* **Interface Graphique (GUI) :** **Iced** — Offre une interface native et fluide sur Windows, macOS et Linux.


* **Réseau & P2P :** **Tokio** (asynchrone) et **WebRTC** (réseau maillé P2P natif, audio et franchissement NAT).



## ⚖️ Licence & Indépendance

Kakolookiyam est open-source et distribué sous la **GNU Affero General Public License v3.0 (AGPLv3)**. Cela garantit que l'architecture réseau reste transparente, auditable et fermement protégée contre toute appropriation commerciale propriétaire.

## 💖 Soutenir le Projet

Kakolookiyam est conçu pour les citoyens, pas pour les courtiers en données. Si cet outil garantit votre liberté numérique, soutenez son développement indépendant :
👉 **[Sponsoriser Altayr sur GitHub](https://github.com/sponsors/AItayr)**

---