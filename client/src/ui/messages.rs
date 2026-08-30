#[derive(Debug, Clone)]
pub enum Message {
    // --- SESSION & AUTHENTIFICATION ---
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

    // --- APPELS & RÉSEAU ---
    PeerIdChanged(String),
    ConnectClicked,
    CallContact(String),
    AcceptCall(String, String),
    RejectCall(String),
    HangUpCall,
    ToggleMute,
    NetworkEvent(String),

    // --- CHAT & NAVIGATION ---
    SelectChat(String),
    DeselectChat,
    ChatInputChanged(String),
    SendChatMessage,
    CopyIdClicked,
    CopyContactId(String),

    // --- GESTION DES GROUPES ---
    NewGroupInputChanged(String),
    CreateGroup,
    NewMemberInputChanged(String),
    AddMemberToGroup,
    AddSpecificMemberToGroup(String),
    DeleteGroup,
    OpenGroupOptions,
    CloseGroupOptions,

    // --- MÉDIAS & FICHIERS ---
    OpenFileDialog,
    FileSelected(Option<String>),
    FileRead(Option<(String, Vec<u8>)>),
    OpenMedia(String, [u8; 32], String),
    MediaSaved(String),
    PreviewMedia(String, [u8; 32]),
    PreviewMediaLoaded(Option<Vec<u8>>),
    ClosePreview,

    // --- PARAMÈTRES & INTERFACE ---
    OpenSettings,
    CloseSettings,
    ToggleTheme,
    MicSelected(String),
    SpeakerSelected(String),
    ToggleLegal(String),
}