#[derive(Debug, Clone)]
pub enum Message {
    GoToCreateAccount,
    GoToLogin,
    BackToWelcome,
    LockSession,
    ForceDisconnect(String),
    TickInactivity,
    ResetInactivity,

    PseudoChanged(String),
    PasswordChanged(String),
    PasswordConfirmChanged(String),

    SubmitCreateAccount,
    SubmitLogin,

    PeerIdChanged(String),
    ConnectClicked,
    CallContact(String),

    AcceptCall(String, String),
    RejectCall(String),

    HangUpCall,
    ToggleMute,

    SelectChat(String),
    DeselectChat,

    OpenFileDialog,
    FileSelected(Option<String>),
    FileRead(Option<(String, Vec<u8>)>),

    // --- NOUVEAU : Ouverture et Déchiffrement des Fichiers ---
    OpenMedia(String, [u8; 32], String),
    MediaSaved(String),

    ChatInputChanged(String),
    SendChatMessage,

    NetworkEvent(String),
    CopyIdClicked,
}