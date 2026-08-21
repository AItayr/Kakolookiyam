#[derive(Debug, Clone)]
pub enum Message {
    GoToCreateAccount,
    GoToLogin,
    BackToWelcome,
    LockSession,
    TickInactivity,
    ResetInactivity, // <-- NOUVEAU : Pour remettre le compteur à zéro

    PseudoChanged(String),
    PasswordChanged(String),
    PasswordConfirmChanged(String),

    SubmitCreateAccount,
    SubmitLogin,

    PeerIdChanged(String),
    ConnectClicked,
    CallContact(String),
    NetworkEvent(String),
    CopyIdClicked,
}