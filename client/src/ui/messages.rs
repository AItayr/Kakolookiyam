#[derive(Debug, Clone)]
pub enum Message {
    GoToCreateAccount,
    GoToLogin,
    BackToWelcome,
    LockSession,
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

    // --- NOUVEAU : Messages pour le Chat P2P ---
    ChatInputChanged(String),
    SendChatMessage,

    NetworkEvent(String),
    CopyIdClicked,
}