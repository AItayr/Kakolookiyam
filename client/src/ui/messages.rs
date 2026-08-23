#[derive(Debug, Clone)]
pub enum Message {
    GoToCreateAccount,
    GoToLogin,
    BackToWelcome,
    LockSession,
    ForceDisconnect(String), // NOUVEAU : Message d'erreur personnalisé
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

    ChatInputChanged(String),
    SendChatMessage,

    NetworkEvent(String),
    CopyIdClicked,
}