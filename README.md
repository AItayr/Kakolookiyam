# Kakolookiyam 

*Connected together. Invisible to the rest.*

A lightweight, high-performance, and ultra-secure cross-platform peer-to-peer (P2P "Full Mesh") communication application. Built natively in Rust without web technologies to ensure maximum performance and absolute user data privacy.

*Supports: Windows, macOS, Linux | Languages: English, Français, العربية (Arabic)*

## Security & Architecture Philosophy (Zero-Knowledge)

This project adopts a strict **zero-trust, zero-trace** design:

* **No Network Persistence:** The blind signaling server and our private, self-hosted Zero-Trace TURN relay handle initial connection matching and NAT traversal anonymously without recording IP addresses or metadata. This completely severs reliance on Big Tech infrastructure (like Google STUN), guaranteeing that your IP addresses are never logged by third parties.


* **Local Vault:** Your identity, contacts, and chat history are encrypted locally using **ChaCha20Poly1305** and **Argon2**.


* **RAM-Only Processing:** Sensitive data and media previews reside strictly in temporary memory (RAM) and are wiped out immediately upon locking the vault.


* **End-to-End Encryption (E2EE):** All direct P2P streams and heavy file transfers (chunked with ACK) are strictly encrypted.


## Technology Stack

* **Language:** **Rust** - Chosen for extreme performance, zero garbage collector, and absolute memory safety.
* **User Interface (GUI):** **Iced** - Provides a streamlined, native interface.
* **Networking & P2P:** **Tokio** (async) and **WebRTC** (native P2P mesh network, audio, and NAT traversal).

## Credits
Special thanks to **Constance Persad** for the design of the Kakolookiyam logo.

## License & Independence

Kakolookiyam is proudly open-source and distributed under the **GNU Affero General Public License v3.0 (AGPLv3)**. This guarantees that the network architecture remains transparent, auditable, and fiercely protected against proprietary corporate appropriation.

## 💛 Support the Project
If this tool guarantees your digital freedom, consider supporting its independent development:
🎁 **[Sponsor Altayr on GitHub](https://github.com/sponsors/AItayr)**


---

# Kakolookiyam

*Connectés entre vous. Invisibles pour le reste.*

Une application de communication pair-à-pair (P2P "Full Mesh") multiplateforme, légère et ultra-sécurisée. Conçue entièrement en natif avec Rust, sans technologies web, pour garantir des performances maximales et une confidentialité absolue.

*Compatibilité : Windows, macOS, Linux | Langues : Anglais, Français, Arabe (العربية)*

## Philosophie de Sécurité (Zero-Knowledge)

Ce projet adopte une architecture stricte **zéro-confiance, zéro-trace** :

* **Aucune persistance réseau :** Le serveur de signalisation aveugle et notre propre relais privé TURN (Zéro-Trace) gèrent la mise en relation et le franchissement des NAT de manière anonyme. Cela élimine purement et simplement toute dépendance aux serveurs mondiaux des géants de la tech (comme le STUN de Google), empêchant toute collecte technique (logs) de vos adresses IP en arrière-plan.


* **Coffre-fort local :** Votre identité, vos contacts et votre historique sont chiffrés localement via **ChaCha20Poly1305** et **Argon2**.


* **Traitement exclusif en RAM :** Les données sensibles et les aperçus de médias résident uniquement en mémoire vive et sont purgés immédiatement au verrouillage.


* **Chiffrement de bout en bout (E2EE) :** Tous les flux P2P directs et les transferts de fichiers lourds sont strictement chiffrés.


## Stack Technique

* **Langage :** **Rust** - Choisi pour ses performances extrêmes, l'absence de garbage collector et sa sécurité mémoire absolue.
* **Interface Graphique (GUI) :** **Iced** - Offre une interface native et fluide.
* **Réseau & P2P :** **Tokio** (asynchrone) et **WebRTC** (réseau maillé P2P natif, audio et franchissement NAT).

## Crédits
Un remerciement tout particulier à **Constance Persad** pour la création et le design du logo Kakolookiyam.

## Licence & Indépendance

Kakolookiyam est open-source et distribué sous la **GNU Affero General Public License v3.0 (AGPLv3)**. Cela garantit que l'architecture réseau reste transparente, auditable et fermement protégée contre toute appropriation commerciale propriétaire.

## 💛 Soutenir le Projet
Si cet outil garantit votre liberté numérique, soutenez son développement indépendant :
🎁 **[Sponsoriser Altayr sur GitHub](https://github.com/sponsors/AItayr)**


---

# Kakolookiyam (كاكولوكيام)

*متصلون معاً. مخفيون عن الباقين.*

تطبيق اتصال مباشر من نظير إلى نظير (P2P "Full Mesh") عبر منصات متعددة، خفيف الوزن وفائق الاستقرار. تم بناؤه بالكامل بلغة Rust دون تقنيات الويب (Web Technologies) لضمان أقصى درجات الأداء والسرية المطلقة لبيانات المستخدم.

*يدعم: Windows، macOS، Linux | اللغات: الإنجليزية، الفرنسية، العربية*

## فلسفة الأمن والمعمارية (Zero-Knowledge)

يتبنى هذا المشروع تصميماً صارماً يعتمد على **انعدام الثقة وانعدام الأثر (Zero-Trust, Zero-Trace)**:

* **لا أثر على الشبكة:** يتولى خادم الإشارات الأعمى وخادم (TURN) الخاص والآمن التعامل مع طلبات الاتصال الأولية وتجاوز جدار الحماية (NAT) بشكل مجهول دون تسجيل أي عناوين IP أو بيانات وصفية (Metadata). هذا يلغي تماماً الحاجة للاعتماد على خوادم شركات التكنولوجيا الكبرى (مثل Google STUN)، مما يضمن عدم تتبع أي طرف ثالث لعناوين IP الخاصة بكم.


* **الخزنة المحلية:** يتم تشفير هويتك وجهات اتصالك وسجل محادثاتك محلياً باستخدام **ChaCha20Poly1305** و **Argon2**.


* **المعالجة في ذاكرة الوصول العشوائي (RAM) فقط:** تقتصر إقامة البيانات الحساسة ومعاينات الوسائط على الذاكرة المؤقتة، ويتم مسحها فوراً عند قفل الخزنة.


* **التشفير من النهاية إلى النهاية (E2EE):** يتم تشفير جميع تدفقات P2P المباشرة ونقل الملفات الثقيلة بصرامة تامة.


## التقنيات المستخدمة

* **لغة البرمجة:** **Rust** - اختيرت لأدائها الفائق، انعدام حاوي القمامة (Garbage Collector)، وأمنها المطلق للذاكرة.
* **واجهة المستخدم (GUI):** **Iced** - يوفر واجهة أصلية وسلسة.
* **الشبكات و P2P:** **Tokio** للبرمجة غير المتزامنة و **WebRTC** لاتصالات P2P والصوت وتخطي شبكات NAT.

## شكر وتقدير
شكر خاص لـ **Constance Persad** على تصميم شعار Kakolookiyam.

## الترخيص والاستقلالية

Kakolookiyam فخور بكونه مفتوح المصدر ويوزع تحت ترخيص **GNU Affero General Public License v3.0 (AGPLv3)**. يضمن ذلك بقاء معمارية الشبكة شفافة وقابلة للتدقيق ومحمية بشراسة ضد أي احتكار تجاري.

## 💛 ادعم المشروع
إذا كان هذا البرنامج يضمن لك حريتك الرقمية، فكر في دعم تطويره المستقل:
🎁 **[دعم Altayr على GitHub](https://github.com/sponsors/AItayr)**
