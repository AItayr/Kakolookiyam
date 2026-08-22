use iced::widget::{button, column, row, text, text_input, Container, Scrollable};
use iced::{Alignment, Element};
use crate::crypto;
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn view_dashboard(&self) -> Element<'_, Message> {
        let vd = self.vault_data.as_ref().unwrap();
        let my_id = crypto::derive_public_id(&vd.private_key);

        let title = text(format!("🛡️ Bonjour {} !", vd.pseudo)).size(28);
        let identity_row = row![text(format!("🔑 Mon ID : {}", my_id)).size(16), button("Copier").on_press(Message::CopyIdClicked)].spacing(10);
        let status = text(&self.status_message);

        let mut contacts_col = column![text("📔 Vos Contacts").size(20)].spacing(10);
        for (id, pseudo) in &vd.contacts {
            let contact_row = row![
                text(format!("👤 {}", pseudo)).size(16),
                button("📞 Appeler").on_press(Message::CallContact(id.clone()))
            ].spacing(15).align_items(Alignment::Center);
            contacts_col = contacts_col.push(contact_row);
        }

        let input = text_input("Entrer l'ID (Clé Publique) d'un nouvel ami...", &self.peer_id_input).on_input(Message::PeerIdChanged).padding(10);
        let connect_button = button("Lancer l'appel sécurisé").on_press(Message::ConnectClicked);
        let lock_button = button("🔒 Verrouiller / Se déconnecter").on_press(Message::LockSession).padding(10);

        let content = column![title, identity_row, status, Scrollable::new(contacts_col), input, connect_button, lock_button]
            .spacing(20).padding(40).align_items(Alignment::Center);

        Container::new(content).center_x().center_y().into()
    }
}