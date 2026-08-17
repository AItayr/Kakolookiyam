use iced::widget::{button, column, text, text_input, Container};
use iced::{Application, Command, Element, Theme, Subscription};
use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver};
use tokio::sync::Mutex;
use std::sync::Arc;

// Notre structure Flags contient maintenant les DEUX câbles
pub struct Flags {
    pub tx_network: UnboundedSender<String>,
    pub rx_network: UnboundedReceiver<String>,
}

pub struct KakolookiyamApp {
    peer_id_input: String,
    status_message: String,
    tx_network: UnboundedSender<String>,
    // On emballe le récepteur dans un Arc<Mutex> pour pouvoir l'écouter en boucle
    rx_network: Arc<Mutex<Option<UnboundedReceiver<String>>>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PeerIdChanged(String),
    ConnectClicked,
    NetworkEvent(String), // Nouveau message pour mettre à jour l'UI !
}

impl Application for KakolookiyamApp {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = Flags;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        (
            Self {
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
            Message::PeerIdChanged(val) => {
                self.peer_id_input = val;
            }
            Message::ConnectClicked => {
                self.status_message = format!("🔗 Génération de l'appel vers : {}...", self.peer_id_input);
                let _ = self.tx_network.send(self.peer_id_input.clone());
            }
            // Quand le réseau nous parle, on met simplement à jour le texte !
            Message::NetworkEvent(msg) => {
                self.status_message = msg;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("🛡️ Kakolookiyam P2P").size(28);
        let status = text(&self.status_message);

        let input = text_input("Entrer l'identifiant de l'ami...", &self.peer_id_input)
            .on_input(Message::PeerIdChanged)
            .padding(10);

        let connect_button = button("Lancer l'appel")
            .on_press(Message::ConnectClicked);

        let content = column![
            title,
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

    // L'oreille de notre interface : elle écoute le câble en continu
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
                    None => std::future::pending().await, // On patiente s'il n'y a rien
                }
            }
        )
    }
}