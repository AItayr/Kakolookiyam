use iced::{Font, font::{Family, Weight, Stretch, Style}};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Language {
    #[default]
    Fr,
    En,
    Ar,
}

pub fn app_font(lang: &Language) -> Font {
    match lang {
        Language::Ar => Font {
            family: Family::Name("Reem Kufi"),
            weight: Weight::Normal,
            stretch: Stretch::Normal,
            style: Style::Normal,
        },
        _ => Font {
            family: Family::Name("Cinzel"),
            weight: Weight::Normal,
            stretch: Stretch::Normal,
            style: Style::Normal,
        },
    }
}

pub fn t(lang: &Language, key: &str) -> String {
    let text = match (lang, key) {
        // Bouton de langue rapide
        (Language::Fr, "lang_toggle") => "English",
        (Language::En, "lang_toggle") => "العربية",
        (Language::Ar, "lang_toggle") => "Français",

        // Accueil & Authentification
        (Language::Fr, "welcome_sub") => "Connectés entre vous. Invisibles pour le reste.",
        (Language::En, "welcome_sub") => "Connected together. Invisible to the rest.",
        (Language::Ar, "welcome_sub") => "متصلون ببعضكم. مخفيون عن البقية.",
        (Language::Fr, "btn_create") => "Créer un nouveau compte (Coffre-fort local)",
        (Language::En, "btn_create") => "Create a new account (Local Vault)",
        (Language::Ar, "btn_create") => "إنشاء حساب جديد (خزنة محلية)",
        (Language::Fr, "btn_login") => "Se connecter (Déverrouiller un compte existant)",
        (Language::En, "btn_login") => "Login (Unlock an existing account)",
        (Language::Ar, "btn_login") => "تسجيل الدخول (فتح حساب موجود)",
        (Language::Fr, "btn_back") => "Retour",
        (Language::En, "btn_back") => "Back",
        (Language::Ar, "btn_back") => "رجوع",

        (Language::Fr, "login_title") => "Déverrouiller le coffre-fort",
        (Language::En, "login_title") => "Unlock Vault",
        (Language::Ar, "login_title") => "فتح الخزنة",
        (Language::Fr, "login_pseudo_placeholder") => "Votre pseudo...",
        (Language::En, "login_pseudo_placeholder") => "Your pseudo...",
        (Language::Ar, "login_pseudo_placeholder") => "الاسم المستعار...",
        (Language::Fr, "login_password_placeholder") => "Votre mot de passe...",
        (Language::En, "login_password_placeholder") => "Your password...",
        (Language::Ar, "login_password_placeholder") => "كلمة المرور...",
        (Language::Fr, "btn_submit_login") => "Déverrouiller",
        (Language::En, "btn_submit_login") => "Unlock",
        (Language::Ar, "btn_submit_login") => "فتح",

        (Language::Fr, "create_vault_title") => "Créer un coffre-fort",
        (Language::En, "create_vault_title") => "Create a Vault",
        (Language::Ar, "create_vault_title") => "إنشاء خزنة",
        (Language::Fr, "create_vault_info") => "Votre mot de passe chiffrera votre clé privée et votre pseudo.",
        (Language::En, "create_vault_info") => "Your password will encrypt your private key and pseudo.",
        (Language::Ar, "create_vault_info") => "كلمة المرور الخاصة بك ستقوم بتشفير مفتاحك الخاص واسمك المستعار.",
        (Language::Fr, "pseudo_placeholder") => "Choisissez un pseudo...",
        (Language::En, "pseudo_placeholder") => "Choose a pseudo...",
        (Language::Ar, "pseudo_placeholder") => "اختر اسماً مستعاراً...",
        (Language::Fr, "password_new_placeholder") => "Nouveau mot de passe (min. 12 car., Maj, Min, Chiffre, Spécial)...",
        (Language::En, "password_new_placeholder") => "New password (min. 12 chars, Upper, Lower, Number, Special)...",
        (Language::Ar, "password_new_placeholder") => "كلمة مرور جديدة (الحد الأدنى 12 حرف، كبير، صغير، رقم، خاص)...",
        (Language::Fr, "password_confirm_placeholder") => "Confirmez le mot de passe...",
        (Language::En, "password_confirm_placeholder") => "Confirm password...",
        (Language::Ar, "password_confirm_placeholder") => "تأكيد كلمة المرور...",
        (Language::Fr, "btn_submit_create") => "Créer et Chiffrer",
        (Language::En, "btn_submit_create") => "Create and Encrypt",
        (Language::Ar, "btn_submit_create") => "إنشاء وتشفير",
        (Language::Fr, "error") => "Erreur",
        (Language::En, "error") => "Error",
        (Language::Fr, "vault_path_info") => "Si vous souhaitez supprimer votre compte et vos données, le dossier se trouve dans : ",
        (Language::En, "vault_path_info") => "If you wish to delete your account and data, the directory is located at : ",
        (Language::Ar, "vault_path_info") => "إذا كنت ترغب في حذف حسابك وبياناتك، فإن المجلد موجود في : ",

        (Language::Ar, "error") => "خطأ",

        // Paramètres
        (Language::Fr, "settings_title") => "PARAMÈTRES DU COFFRE",
        (Language::En, "settings_title") => "VAULT SETTINGS",
        (Language::Ar, "settings_title") => "إعدادات الخزنة",
        (Language::Fr, "settings_general") => "Général",
        (Language::En, "settings_general") => "General",
        (Language::Ar, "settings_general") => "عام",
        (Language::Fr, "settings_lang") => "Basculer la langue (Fr/En/Ar)",
        (Language::En, "settings_lang") => "Toggle Language (En/Fr/Ar)",
        (Language::Ar, "settings_lang") => "تغيير اللغة",
        (Language::Fr, "settings_theme") => "Basculer le thème (Clair/Sombre)",
        (Language::En, "settings_theme") => "Toggle Theme (Light/Dark)",
        (Language::Ar, "settings_theme") => "تغيير المظهر (فاتح/داكن)",
        (Language::Fr, "settings_audio") => "Matériel & Audio",
        (Language::En, "settings_audio") => "Hardware & Audio",
        (Language::Ar, "settings_audio") => "الأجهزة والصوت",
        (Language::Fr, "settings_mic") => "Microphone (Entrée)",
        (Language::En, "settings_mic") => "Microphone (Input)",
        (Language::Ar, "settings_mic") => "الميكروفون (إدخال)",
        (Language::Fr, "settings_speaker") => "Haut-parleurs (Sortie)",
        (Language::En, "settings_speaker") => "Speakers (Output)",
        (Language::Ar, "settings_speaker") => "مكبرات الصوت (إخراج)",
        (Language::Fr, "device_default") => "Périphérique par défaut (Système)",
        (Language::En, "device_default") => "Default Device (System)",
        (Language::Ar, "device_default") => "الجهاز الافتراضي (النظام)",
        (Language::Fr, "device_virtual_in") => "Entrée virtuelle",
        (Language::En, "device_virtual_in") => "Virtual Input",
        (Language::Ar, "device_virtual_in") => "إدخال وهمي",
        (Language::Fr, "device_virtual_out") => "Sortie virtuelle",
        (Language::En, "device_virtual_out") => "Virtual Output",
        (Language::Ar, "device_virtual_out") => "إخراج وهمي",
        (Language::Fr, "btn_close") => "Fermer",
        (Language::En, "btn_close") => "Close",
        (Language::Ar, "btn_close") => "إغلاق",

        // Légal
        (Language::Fr, "settings_legal") => "Légal, Mentions & Crédits",
        (Language::En, "settings_legal") => "Legal, Notices & Credits",
        (Language::Ar, "settings_legal") => "قانوني، إشعارات وائتمانات",

        (Language::Fr, "settings_cgu_btn") => "Licence & Infrastructure",
        (Language::En, "settings_cgu_btn") => "License & Infrastructure",
        (Language::Ar, "settings_cgu_btn") => "الترخيص والبنية التحتية",

        (Language::Fr, "cgu_title") => "Licence AGPLv3 & Réseau",
        (Language::En, "cgu_title") => "AGPLv3 License & Network",
        (Language::Ar, "cgu_title") => "رخصة AGPLv3 والشبكة",

        (Language::Fr, "cgu_text") => "Kakolookiyam est distribué sous la GNU Affero General Public License v3.0 (AGPLv3). \n\nL'architecture d'échange est bâtie sur un modèle asymétrique Zéro-Trust :\n- L'établissement des tunnels (Signaling WSS) s'effectue via un relais masqué (Reverse-Proxy) sous juridiction Suisse (.ch).\n- Le trafic Voix & Fichiers, géré en P2P (WebRTC), subit un \"Forçage Relais\" (Relay Policy) strict vers l'infrastructure Cloud d'Oracle [cite: 1]. La nature même de votre adresse réseau est effacée.\n\nCréation de l'identité visuelle et du logo par Constance PERSAD.\n\nConception sonore et musiques composées par Toxare (https://open.spotify.com/intl-fr/artist/5AFESJ1lGjK6hkCJLY8iwn?si=4kiT3AwnTw6OwlfoRpUU3A | teumaaa.pro@gmail.com). Toutes les œuvres sonores sont protégées à vie sur la blockchain Tezos via le service MusicStart (URights / Sacem).",
        (Language::En, "cgu_text") => "Kakolookiyam is distributed under the GNU AGPLv3 license. \n\nThe exchange architecture is built on a Zero-Trust asymmetric model:\n- Tunnel establishment (WSS Signaling) is managed through a masked relay (Reverse-Proxy) under Swiss jurisdiction (.ch).\n- Voice & File traffic (WebRTC P2P) undergoes strict \"Relay Policy\" enforcement through Oracle Cloud infrastructure [cite: 1]. The very nature of your network address is wiped.\n\nVisual identity and logo designed by Constance PERSAD.\n\nSound design and music composed by Toxare (https://open.spotify.com/intl-fr/artist/5AFESJ1lGjK6hkCJLY8iwn?si=4kiT3AwnTw6OwlfoRpUU3A | teumaaa.pro@gmail.com). All audio works are protected for life on the Tezos blockchain via the MusicStart service (URights / Sacem).",
        (Language::Ar, "cgu_text") => "يتم توزيع Kakolookiyam تحت رخصة GNU AGPLv3، مما يضمن استقلالية الكود. \n\nيتم تأمين إشارات WSS بواسطة وكيل عكسي تحت الولاية القضائية السويسرية (.ch).\nيعمل مرحل P2P الأعمى على البنية التحتية لـ Oracle، ويخضع لاتفاقية خدمات Oracle السحابية[cite: 1].\n\n  الهوية البصرية والشعار من تصميم Constance PERSAD.\n\nتصميم الصوت والموسيقى من تأليف Toxare (https://open.spotify.com/intl-fr/artist/5AFESJ1lGjK6hkCJLY8iwn?si=4kiT3AwnTw6OwlfoRpUU3A | teumaaa.pro@gmail.com). جميع الأعمال الصوتية محمية مدى الحياة على بلوكشين Tezos عبر خدمة MusicStart (URights / Sacem).",

        (Language::Fr, "privacy_title") => "Philosophie Zéro-Trace",
        (Language::En, "privacy_title") => "Zero-Knowledge Philosophy",
        (Language::Ar, "privacy_title") => "فلسفة انعدام المعرفة",

        (Language::Fr, "privacy_text") => "Connectés entre vous. Invisibles pour le reste. \n\n Identité chiffrée (ChaCha20Poly1305/Argon2) et traitements exclusifs en RAM, purgés au verrouillage (mlock / Zeroize). \n\nAucun serveur tiers ne stocke vos métadonnées.",
        (Language::En, "privacy_text") => "Connected together. Invisible to the rest. \n\nIdentity is strictly encrypted locally (ChaCha20Poly1305/Argon2) and processing is RAM-only, securely wiped upon locking (mlock / Zeroize). \n\nNo third-party servers store your metadata.",
        (Language::Ar, "privacy_text") => "متصلون ببعضكم. مخفيون عن البقية. \n\nيتم تشفير الهوية محلياً (ChaCha20Poly1305/Argon2) والمعالجة تتم في RAM فقط، وتُمسح بأمان عند القفل. \n\nلا تخزن خوادم الطرف الثالث بياناتك الوصفية.",

        (Language::Fr, "ofl_title") => "Composants Open-Source",
        (Language::En, "ofl_title") => "Open-Source Components",
        (Language::Ar, "ofl_title") => "مكونات مفتوحة المصدر",

        (Language::Fr, "ofl_text") => "Application native en Rust (Iced, Tokio, WebRTC). La police 'Cinzel' est sous SIL Open Font License (OFL).\n\n  Soutenez le projet libre sur GitHub (Sponsors : Altayr).",
        (Language::En, "ofl_text") => "Built natively in Rust (Iced, Tokio, WebRTC). 'Cinzel' font is distributed under the SIL Open Font License (OFL).\n\n  Support the independent project on GitHub (Sponsors: Altayr).",
        (Language::Ar, "ofl_text") => "مبني بشكل أصلي في Rust (Iced, Tokio, WebRTC). توزع خطوط 'Cinzel' و 'ReemKufi' تحت رخصة SIL المفتوحة (OFL).\n\n  ادعم المشروع المستقل على GitHub (الرعاة: Altayr).",

        // Dashboard (Barre latérale)
        (Language::Fr, "my_id") => "Mon ID",
        (Language::En, "my_id") => "My ID",
        (Language::Ar, "my_id") => "معرفي",
        (Language::Fr, "btn_copy") => "Copier",
        (Language::En, "btn_copy") => "Copy",
        (Language::Ar, "btn_copy") => "نسخ",
        (Language::Fr, "contacts") => "CONTACTS",
        (Language::En, "contacts") => "CONTACTS",
        (Language::Ar, "contacts") => "جهات الاتصال",
        (Language::Fr, "servers") => "SERVEURS",
        (Language::En, "servers") => "SERVERS",
        (Language::Ar, "servers") => "الخوادم",
        (Language::Fr, "new_server") => "Nouveau serveur...",
        (Language::En, "new_server") => "New server...",
        (Language::Ar, "new_server") => "خادم جديد...",
        (Language::Fr, "btn_settings") => "Paramètres",
        (Language::En, "btn_settings") => "Settings",
        (Language::Ar, "btn_settings") => "الإعدادات",
        (Language::Fr, "btn_lock") => "Verrouiller le coffre",
        (Language::En, "btn_lock") => "Lock Vault",
        (Language::Ar, "btn_lock") => "قفل الخزنة",

        // Dashboard (Options Serveur)
        (Language::Fr, "add_public_id") => "Ajouter par ID public...",
        (Language::En, "add_public_id") => "Add by public ID...",
        (Language::Ar, "add_public_id") => "إضافة عبر المعرف العام...",
        (Language::Fr, "quick_add_contacts") => "Ajout rapide (Contacts)",
        (Language::En, "quick_add_contacts") => "Quick add (Contacts)",
        (Language::Ar, "quick_add_contacts") => "إضافة سريعة (جهات الاتصال)",
        (Language::Fr, "btn_add_prefix") => "+ Ajouter",
        (Language::En, "btn_add_prefix") => "+ Add",
        (Language::Ar, "btn_add_prefix") => "+ إضافة",
        (Language::Fr, "btn_add") => "+ Ajouter",
        (Language::En, "btn_add") => "+ Add",
        (Language::Ar, "btn_add") => "+ إضافة",
        (Language::Fr, "all_contacts_in_server") => "Tous vos contacts sont déjà dans ce serveur.",
        (Language::En, "all_contacts_in_server") => "All your contacts are already in this server.",
        (Language::Ar, "all_contacts_in_server") => "جميع جهات الاتصال الخاصة بك موجودة بالفعل في هذا الخادم.",
        (Language::Fr, "btn_delete_server") => "Supprimer le serveur",
        (Language::En, "btn_delete_server") => "Delete server",
        (Language::Ar, "btn_delete_server") => "حذف الخادم",
        (Language::Fr, "btn_leave_server") => "Quitter le serveur",
        (Language::En, "btn_leave_server") => "Leave server",
        (Language::Ar, "btn_leave_server") => "مغادرة الخادم",
        (Language::Fr, "options_prefix") => "Options :",
        (Language::En, "options_prefix") => "Options:",
        (Language::Ar, "options_prefix") => "خيارات:",
        (Language::Fr, "invite_member") => "Inviter un membre",
        (Language::En, "invite_member") => "Invite a member",
        (Language::Ar, "invite_member") => "دعوة عضو",

        // Dashboard (Chat)
        (Language::Fr, "btn_return") => "Retour",
        (Language::En, "btn_return") => "Back",
        (Language::Ar, "btn_return") => "رجوع",
        (Language::Fr, "btn_call_group") => "Appeler",
        (Language::En, "btn_call_group") => "Call",
        (Language::Ar, "btn_call_group") => "اتصال",
        (Language::Fr, "btn_join_group") => "Rejoindre",
        (Language::En, "btn_join_group") => "Join",
        (Language::Ar, "btn_join_group") => "انضمام",
        (Language::Fr, "btn_options") => "Options",
        (Language::En, "btn_options") => "Options",
        (Language::Ar, "btn_options") => "خيارات",
        (Language::Fr, "btn_copy_id") => "Copier ID",
        (Language::En, "btn_copy_id") => "Copy ID",
        (Language::Ar, "btn_copy_id") => "نسخ المعرف",
        (Language::Fr, "btn_close_preview") => "Fermer l'aperçu",
        (Language::En, "btn_close_preview") => "Close preview",
        (Language::Ar, "btn_close_preview") => "إغلاق المعاينة",
        (Language::Fr, "btn_extract") => "Extraire & Ouvrir",
        (Language::En, "btn_extract") => "Extract & Open",
        (Language::Ar, "btn_extract") => "استخراج وفتح",
        (Language::Fr, "btn_preview_ram") => "Aperçu RAM",
        (Language::En, "btn_preview_ram") => "RAM Preview",
        (Language::Ar, "btn_preview_ram") => "معاينة RAM",
        (Language::Fr, "no_message") => "Aucun message. Soyez le premier à écrire !",
        (Language::En, "no_message") => "No messages. Be the first to write!",
        (Language::Ar, "no_message") => "لا توجد رسائل. كن أول من يكتب!",
        (Language::Fr, "send_to_prefix") => "Envoyer à",
        (Language::En, "send_to_prefix") => "Send to",
        (Language::Ar, "send_to_prefix") => "إرسال إلى",
        (Language::Fr, "btn_send") => "Envoyer",
        (Language::En, "btn_send") => "Send",
        (Language::Ar, "btn_send") => "إرسال",
        (Language::Fr, "welcome_chat_1") => "BIENVENUE DANS KAKOLOOKIYAM",
        (Language::En, "welcome_chat_1") => "WELCOME TO KAKOLOOKIYAM",
        (Language::Ar, "welcome_chat_1") => "مرحباً بكم في KAKOLOOKIYAM",
        (Language::Fr, "welcome_chat_2") => "VOTRE LOGICIEL D'ECHANGE SÉCURISÉ",
        (Language::En, "welcome_chat_2") => "YOUR SECURE EXCHANGE SOFTWARE",
        (Language::Ar, "welcome_chat_2") => "برنامجكم للتبادل الآمن",
        (Language::Fr, "select_contact") => "Sélectionnez un contact ou un serveur à gauche.",
        (Language::En, "select_contact") => "Select a contact or a server on the left.",
        (Language::Ar, "select_contact") => "حدد جهة اتصال أو خادم من اليسار.",
        (Language::Fr, "add_public_id_call") => "Ajouter un ID public...",
        (Language::En, "add_public_id_call") => "Add a public ID...",
        (Language::Ar, "add_public_id_call") => "إضافة معرف عام...",
        (Language::Fr, "btn_add_call") => "Ajouter & Appeler",
        (Language::En, "btn_add_call") => "Add & Call",
        (Language::Ar, "btn_add_call") => "إضافة واتصال",

        (Language::Fr, "member_left") => "Le membre {name} a quitté le groupe.",
        (Language::En, "member_left") => "Member {name} has left the group.",
        (Language::Ar, "member_left") => " غادر العضو {name} المجموعة.",

        // Appels
        (Language::Fr, "banner_call_active") => "Appel en cours",
        (Language::En, "banner_call_active") => "Active call",
        (Language::Ar, "banner_call_active") => "مكالمة جارية",
        (Language::Fr, "call_secure_with") => "En communication sécurisée avec",
        (Language::En, "call_secure_with") => "In secure communication with",
        (Language::Ar, "call_secure_with") => "في اتصال آمن مع",
        (Language::Fr, "mic_enable") => "Activer le micro",
        (Language::En, "mic_enable") => "Enable Mic",
        (Language::Ar, "mic_enable") => "تفعيل الميكروفون",
        (Language::Fr, "mic_disable") => "Couper le micro (Mute)",
        (Language::En, "mic_disable") => "Mute Mic",
        (Language::Ar, "mic_disable") => "كتم الميكروفون",
        (Language::Fr, "btn_hangup") => "Raccrocher",
        (Language::En, "btn_hangup") => "Hang up",
        (Language::Ar, "btn_hangup") => "إنهاء المكالمة",
        (Language::Fr, "call_incoming") => "Appel entrant !",
        (Language::En, "call_incoming") => "Incoming call!",
        (Language::Ar, "call_incoming") => "مكالمة واردة!",
        (Language::Fr, "wants_to_talk") => "souhaite communiquer avec vous.",
        (Language::En, "wants_to_talk") => "wants to communicate with you.",
        (Language::Ar, "wants_to_talk") => "يريد التواصل معك.",
        (Language::Fr, "auto_reject") => "Rejet automatique dans",
        (Language::En, "auto_reject") => "Auto reject in",
        (Language::Ar, "auto_reject") => "رفض تلقائي في",
        (Language::Fr, "btn_accept") => "Décrocher",
        (Language::En, "btn_accept") => "Accept",
        (Language::Ar, "btn_accept") => "رد",
        (Language::Fr, "btn_reject") => "Rejeter",
        (Language::En, "btn_reject") => "Reject",
        (Language::Ar, "btn_reject") => "رفض",
        (Language::Fr, "chat_ephemeral") => "Chat Éphémère (Zéro-Trace)",
        (Language::En, "chat_ephemeral") => "Ephemeral Chat (Zero-Trace)",
        (Language::Ar, "chat_ephemeral") => "دردشة مؤقتة (صفر أثر)",
        (Language::Fr, "chat_placeholder_stealth") => "Écrivez un message furtif...",
        (Language::En, "chat_placeholder_stealth") => "Write a stealth message...",
        (Language::Ar, "chat_placeholder_stealth") => "اكتب رسالة خفية...",

        
        // --- NEW STATUS MESSAGES FIX ---
        (Language::Fr, "status_call_missed") => " Appel manqué.",
        (Language::En, "status_call_missed") => " Missed call.",
        (Language::Ar, "status_call_missed") => " مكالمة فائتة.",

        (Language::Fr, "err_choose_pseudo") => "Veuillez choisir un pseudo.",
        (Language::En, "err_choose_pseudo") => "Please choose a username.",
        (Language::Ar, "err_choose_pseudo") => "يرجى اختيار اسم مستخدم.",

        (Language::Fr, "err_profile_exists") => "Ce profil existe déjà sur cet ordinateur.",
        (Language::En, "err_profile_exists") => "This profile already exists on this computer.",
        (Language::Ar, "err_profile_exists") => "هذا الملف الشخصي موجود بالفعل على هذا الكمبيوتر.",

        (Language::Fr, "err_passwords_match") => "Mots de passe distincts.",
        (Language::En, "err_passwords_match") => "Passwords do not match.",
        (Language::Ar, "err_passwords_match") => "كلمات المرور غير متطابقة.",

        (Language::Fr, "err_enter_pseudo") => "Veuillez entrer votre pseudo.",
        (Language::En, "err_enter_pseudo") => "Please enter your username.",
        (Language::Ar, "err_enter_pseudo") => "الرجاء إدخال اسم المستخدم الخاص بك.",

        (Language::Fr, "err_enter_password") => "Veuillez entrer un mot de passe.",
        (Language::En, "err_enter_password") => "Please enter a password.",
        (Language::Ar, "err_enter_password") => "الرجاء إدخال كلمة مرور.",

        (Language::Fr, "status_call_waiting") => " En attente de l'interlocuteur...",
        (Language::En, "status_call_waiting") => " Waiting for the interlocutor...",
        (Language::Ar, "status_call_waiting") => " في انتظار المتصل...",

        (Language::Fr, "status_call_group") => " Conférence de groupe en cours...",
        (Language::En, "status_call_group") => " Group conference in progress...",
        (Language::Ar, "status_call_group") => " مؤتمر جماعي قيد التقدم...",

        (Language::Fr, "status_call_secure") => " Connexion sécurisée en cours...",
        (Language::En, "status_call_secure") => " Securing connection...",
        (Language::Ar, "status_call_secure") => " جارٍ تأمين الاتصال...",

        (Language::Fr, "status_call_rejected") => "❌ Appel rejeté.",
        (Language::En, "status_call_rejected") => "❌ Call rejected.",
        (Language::Ar, "status_call_rejected") => "❌ مكالمة مرفوضة.",

        (Language::Fr, "status_call_ended") => "Appel terminé.",
        (Language::En, "status_call_ended") => "Call ended.",
        (Language::Ar, "status_call_ended") => "انتهت المكالمة.",

        (Language::Fr, "status_contact_added") => "✅ Contact ajouté !",
        (Language::En, "status_contact_added") => "✅ Contact added!",
        (Language::Ar, "status_contact_added") => "✅ تمت إضافة جهة الاتصال!",

        (Language::Fr, "status_request_rejected") => "❌ Demande rejetée.",
        (Language::En, "status_request_rejected") => "❌ Request rejected.",
        (Language::Ar, "status_request_rejected") => "❌ تم رفض الطلب.",

        (Language::Fr, "status_id_copied") => "✅ ID copié dans le presse-papiers !",
        (Language::En, "status_id_copied") => "✅ ID copied to clipboard!",
        (Language::Ar, "status_id_copied") => "✅ تم نسخ المعرف إلى الحافظة!",

        (Language::Fr, "status_member_exists") => "❌ Ce membre est déjà dans le groupe.",
        (Language::En, "status_member_exists") => "❌ This member is already in the group.",
        (Language::Ar, "status_member_exists") => "❌ هذا العضو موجود بالفعل في المجموعة.",

        (Language::Fr, "status_member_invited") => "✅ Membre invité et synchronisé !",
        (Language::En, "status_member_invited") => "✅ Member invited and synchronized!",
        (Language::Ar, "status_member_invited") => "✅ تمت دعوة العضو ومزامنته!",

        (Language::Fr, "status_contact_synced") => "✅ Contact ajouté et synchronisé !",
        (Language::En, "status_contact_synced") => "✅ Contact added and synchronized!",
        (Language::Ar, "status_contact_synced") => "✅ تمت إضافة جهة الاتصال ومزامنتها!",

        (Language::Fr, "status_group_left") => "☑️ Groupe quitté avec succès.",
        (Language::En, "status_group_left") => "☑️ Group successfully left.",
        (Language::Ar, "status_group_left") => "☑️ تم مغادرة المجموعة بنجاح.",

        (Language::Fr, "status_file_too_large") => "❌ Erreur : Le fichier dépasse la limite de 50 Mo.",
        (Language::En, "status_file_too_large") => "❌ Error: The file exceeds the 50 MB limit.",
        (Language::Ar, "status_file_too_large") => "❌ خطأ: يتجاوز الملف حد 50 ميغابايت.",

        (Language::Fr, "status_pwd_loading") => "⏳ Chargement sécurisé de l'aperçu...",
        (Language::En, "status_pwd_loading") => "⏳ Securely loading preview...",
        (Language::Ar, "status_pwd_loading") => "⏳ جاري تحميل المعاينة بشكل آمن...",

        (Language::Fr, "status_media_ram") => "✅ Aperçu média chargé en mémoire RAM (Zéro-Trace).",
        (Language::En, "status_media_ram") => "✅ Media preview loaded into RAM (Zero-Trace).",
        (Language::Ar, "status_media_ram") => "✅ تم تحميل معاينة الوسائط في ذاكرة الوصول العشوائي (صفر أثر).",

        (Language::Fr, "status_media_error") => "❌ Impossible de générer l'aperçu.",
        (Language::En, "status_media_error") => "❌ Unable to generate preview.",
        (Language::Ar, "status_media_error") => "❌ تعذر إنشاء معاينة.",

        (Language::Fr, "error_self_call") => "Vous ne pouvez pas vous appeler vous-même.",
        (Language::En, "error_self_call") => "You cannot call yourself.",
        (Language::Ar, "error_self_call") => "لا يمكنك الاتصال بنفسك.",

        (Language::Fr, "file_received_prefix") => "Fichier reçu",
        (Language::En, "file_received_prefix") => "File received",
        (Language::Ar, "file_received_prefix") => "ملف مستلم",

        (Language::Fr, "btn_accept_file") => "Accepter",
        (Language::En, "btn_accept_file") => "Accept",
        (Language::Ar, "btn_accept_file") => "قبول",

        (Language::Fr, "btn_reject_file") => "Refuser",
        (Language::En, "btn_reject_file") => "Decline",
        (Language::Ar, "btn_reject_file") => "رفض",

        (Language::Fr, "status_media_purged") => " Aperçu fermé (Données purgées de la RAM).",
        (Language::En, "status_media_purged") => " Preview closed (Data purged from RAM).",
        (Language::Ar, "status_media_purged") => " تم إغلاق المعاينة (تم مسح البيانات من ذاكرة الوصول العشوائي).",

        (Language::Fr, "status_remote_busy") => "L'interlocuteur est déjà en ligne (Occupé).",
        (Language::En, "status_remote_busy") => "The interlocutor is already on a call (Busy).",
        (Language::Ar, "status_remote_busy") => "المتصل متصل بالفعل (مشغول).",

        (Language::Fr, "status_server_dissolved") => "🔴 Le créateur a dissous le serveur.",
        (Language::En, "status_server_dissolved") => "🔴 The creator has dissolved the server.",
        (Language::Ar, "status_server_dissolved") => "🔴 حل المؤسس الخادم.",

        (Language::Fr, "status_conf_joined") => "✅ Conférence rejointe : {name}",
        (Language::En, "status_conf_joined") => "✅ Conference joined: {name}",
        (Language::Ar, "status_conf_joined") => "✅ تم الانضمام إلى المؤتمر: {name}",

        (Language::Fr, "status_server_invited") => "✅ Invité dans le serveur {name} !",
        (Language::En, "status_server_invited") => "✅ Invited to server {name}!",
        (Language::Ar, "status_server_invited") => "✅ تمت دعوتك إلى الخادم {name}!",

        (Language::Fr, "status_member_left") => "Un membre a quitté le serveur.",
        (Language::En, "status_member_left") => "A member has left the server.",
        (Language::Ar, "status_member_left") => " غادر عضو الخادم.",

        (Language::Fr, "status_new_request") => "Nouvelle demande de contact en attente !",
        (Language::En, "status_new_request") => "New contact request pending!",
        (Language::Ar, "status_new_request") => "طلب اتصال جديد معلق!",

        (Language::Fr, "status_remote_busy_named") => "📞 {name} est déjà en ligne (Occupé).",
        (Language::En, "status_remote_busy_named") => "📞 {name} is already on a call (Busy).",
        (Language::Ar, "status_remote_busy_named") => "📞 {name} متصل بالفعل (مشغول).",

        (Language::Fr, "status_remote_hangup") => "L'interlocuteur a raccroché.",
        (Language::En, "status_remote_hangup") => "The interlocutor hung up.",
        (Language::Ar, "status_remote_hangup") => "أنهى المتصل المكالمة.",

        (Language::Fr, "status_caller_hangup") => "L'appelant a raccroché.",
        (Language::En, "status_caller_hangup") => "The caller hung up.",
        (Language::Ar, "status_caller_hangup") => "أنهى المتصل المكالمة.",

        (Language::Fr, "status_remote_unavailable") => "L'interlocuteur n'est pas disponible.",
        (Language::En, "status_remote_unavailable") => "The interlocutor is not available.",
        (Language::Ar, "status_remote_unavailable") => "المتصل غير متاح.",

        (Language::Fr, "securing_connection") => "Sécurisation de la connexion",
        (Language::En, "securing_connection") => "Securing connection",
        (Language::Ar, "securing_connection") => "تأمين الاتصال",

        (Language::Fr, "pending_requests_title") => "DEMANDES EN ATTENTE",
        (Language::En, "pending_requests_title") => "PENDING REQUESTS",
        (Language::Ar, "pending_requests_title") => "الطلبات المعلقة",

        
        (Language::Fr, "me_author") => "Moi",
        (Language::En, "me_author") => "Me",
        (Language::Ar, "me_author") => "أنا",

        (Language::Fr, "system_author") => "Système",
        (Language::En, "system_author") => "System",
        (Language::Ar, "system_author") => "النظام",

        (Language::Fr, "msg_missed_call") => "📞 Appel manqué (Ligne occupée)",
        (Language::En, "msg_missed_call") => "📞 Missed call (Line busy)",
        (Language::Ar, "msg_missed_call") => "📞 مكالمة فائتة (الخط مشغول)",

        (Language::Fr, "msg_file_received") => "📎 Fichier reçu :",
        (Language::En, "msg_file_received") => "📎 File received:",
        (Language::Ar, "msg_file_received") => "📎 تم استلام الملف:",

        (Language::Fr, "msg_file_shared") => "📎 Fichier partagé :",
        (Language::En, "msg_file_shared") => "📎 File shared:",
        (Language::Ar, "msg_file_shared") => "📎 الملف المشترك:",

        (Language::Fr, "msg_secure_canal") => "Canal P2P Zéro-Trace sécurisé...",
        (Language::En, "msg_secure_canal") => "P2P Zero-Trace Channel secured...",
        (Language::Ar, "msg_secure_canal") => "تأمين قناة P2P خالية من التتبع...",

        
        (Language::Fr, "ready_to_call") => "Prêt à appeler...",
        (Language::En, "ready_to_call") => "Ready to call...",
        (Language::Ar, "ready_to_call") => "جاهز للاتصال...",
        
        (Language::Fr, "msg_call_ended_duration") => "📞 Appel terminé ({duration})",
        (Language::En, "msg_call_ended_duration") => "📞 Call ended ({duration})",
        (Language::Ar, "msg_call_ended_duration") => "📞 انتهت المكالمة ({duration})",
        (Language::Fr, "status_group_joined") => " Conférence rejointe : {name}",
        (Language::En, "status_group_joined") => " Conference joined: {name}",
        (Language::Ar, "status_group_joined") => " تم الانضمام للمؤتمر: {name}",
        (Language::Fr, "btn_block") => "BLOQUER",
        (Language::En, "btn_block") => "BLOCK",
        (Language::Ar, "btn_block") => "حظر",
        (Language::Fr, "blocked_contacts") => "BLOQUÉS",
        (Language::En, "blocked_contacts") => "BLOCKED",
        (Language::Ar, "blocked_contacts") => "محظور",
        _ => key,




    };
    text.to_string()
}


pub fn app_text<'a>(lang: &Language, content: impl iced::widget::text::IntoFragment<'a>) -> iced::widget::Text<'a> {
    iced::widget::text(content).font(app_font(lang)).shaping(iced::widget::text::Shaping::Advanced)
}