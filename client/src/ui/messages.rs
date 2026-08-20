#[derive(Debug, Clone)]
pub enum Message {
    GoToCreateAccount,
    GoToLogin,
    BackToWelcome,

    PasswordChanged(String),
    PasswordConfirmChanged(String),

    SubmitCreateAccount,
    SubmitLogin,

    PeerIdChanged(String),
    ConnectClicked,
    NetworkEvent(String),
    CopyIdClicked,
}