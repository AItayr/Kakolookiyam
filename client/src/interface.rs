use iced::widget::{button, column, row, text, text_input, Container};
use iced::{clipboard, Application, Command, Element, Theme, Subscription};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct Flags {
    pub tx_network: UnboundedSender<String>,
    pub rx_network: UnboundedReceiver<String>,
    pub my_local_id: String, // On reçoit notre ID au démarrage
}

pub struct KakolookiyamApp {
    my_local_id: String, // On stocke notre ID
    peer_id_input: String,
    status_message: String,
    tx_network: UnboundedSender<String>,
    rx_network: Arc<Mutex<Option<UnboundedReceiver<String>>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PeerIdChanged(String),
    ConnectClicked,
    NetworkEvent(String),
    CopyIdClicked, // Action du bouton Copier
}

impl Application for KakolookiyamApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
                my_local_id: flags.my_local_id, // On assigne l'ID
                peer_id_input: String::new(),
                status_message: "⏳ Prêt à appeler...".to_owned(),
                tx_network: flags.tx_network,
                rx_network: Arc::new(Mutex::new(Some(flags.rx_network))),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Kakolookiyam - Secure P2P")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::PeerIdChanged(val) => self.peer_id_input = val,
            Message::ConnectClicked => {
                self.status_message = format!("🔗 Négociation cryptographique avec : {}...", self.peer_id_input);
                let _ = self.tx_network.send(self.peer_id_input.clone());
            }
            Message::NetworkEvent(msg) => self.status_message = msg,
            Message::CopyIdClicked => {
                // Copie l'ID dans le presse-papiers de Windows/Linux/Mac
                return clipboard::write(self.my_local_id.clone());
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("🛡️ Kakolookiyam P2P").size(28);

        // Affichage de ton ID Cryptographique et du bouton Copier sur la même ligne
        let my_id_display = text(format!("🔑 Mon ID : {}", self.my_local_id)).size(16);
        let copy_button = button("Copier").on_press(Message::CopyIdClicked);
        let identity_row = row![my_id_display, copy_button].spacing(10);

        let status = text(&self.status_message);

        let input = text_input("Entrer l'ID (Clé Publique) de l'ami...", &self.peer_id_input)
            .on_input(Message::PeerIdChanged)
            .padding(10);

        let connect_button = button("Lancer l'appel sécurisé")
            .on_press(Message::ConnectClicked);

        let content = column![
            title,
            identity_row,
            status,
            input,
            connect_button,
        ]
        .spacing(20)
        .padding(40);

        Container::new(content)
            .center_x()
            .center_y()
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        struct NetworkSub;
        iced::subscription::unfold(
            std::any::TypeId::of::<NetworkSub>(),
            self.rx_network.clone(),
            |rx_mutex| async move {
                let msg = {
                    let mut guard = rx_mutex.lock().await;
                    if let Some(rx) = guard.as_mut() {
                        rx.recv().await
                    } else {
                        None
                    }
                };
                match msg {
                    Some(text) => (Message::NetworkEvent(text), rx_mutex),
                    None => std::future::pending().await,
                }
            }
        )
    }
}