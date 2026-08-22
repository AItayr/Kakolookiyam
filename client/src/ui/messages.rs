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

    // NOUVEAU : Commandes pour l'appel en cours
    HangUpCall,
    ToggleMute,

    NetworkEvent(String),
    CopyIdClicked,
}