use iced::widget::{button, column, row, text, text_input, Container, Scrollable, Rule};
use iced::{Alignment, Element, Length, Color};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn view_dashboard(&self) -> Element<'_, Message> {
        let vd = self.vault_data.as_ref().unwrap();

        let mut sidebar = column![
            text(format!("🛡️ {}", vd.pseudo)).size(24),
            row![text("🔑 Mon ID").size(14), button("Copier").on_press(Message::CopyIdClicked).padding(5)].spacing(10),
            Rule::horizontal(10),
        ].spacing(15).width(Length::Fixed(250.0));

        sidebar = sidebar.push(text("📔 CONTACTS").size(16).style(Color::from_rgb(0.5, 0.5, 0.5)));
        for (id, pseudo) in &vd.contacts {
            let is_selected = self.selected_chat.as_ref() == Some(id);
            let text_color = if is_selected { Color::from_rgb(0.2, 0.6, 1.0) } else { Color::BLACK };

            let btn_chat = button(text(format!("👤 {}", pseudo)).size(16).style(text_color))
                .style(iced::theme::Button::Text)
                .on_press(Message::SelectChat(id.clone()));

            sidebar = sidebar.push(row![btn_chat].spacing(10).align_items(Alignment::Center));
        }

        sidebar = sidebar.push(Rule::horizontal(10));
        sidebar = sidebar.push(text("🏘️ SERVEURS (Groupes)").size(16).style(Color::from_rgb(0.5, 0.5, 0.5)));
        for (id, group) in &vd.groups {
            let is_selected = self.selected_chat.as_ref() == Some(id);
            let text_color = if is_selected { Color::from_rgb(0.2, 0.6, 1.0) } else { Color::BLACK };

            sidebar = sidebar.push(
                button(text(format!("# {}", group.name)).size(16).style(text_color))
                    .style(iced::theme::Button::Text)
                    .on_press(Message::SelectChat(id.clone()))
            );
        }

        let sidebar_scroll = Scrollable::new(sidebar).height(Length::Fill);

        let main_content = if let Some(target_id) = &self.selected_chat {
            let target_name = if let Some(p) = vd.contacts.get(target_id) { p.clone() }
                              else if let Some(g) = vd.groups.get(target_id) { g.name.clone() }
                              else { "Inconnu".to_string() };

            let header = row![
                button("⬅️ Retour").on_press(Message::DeselectChat).padding(10),
                Container::new(text(format!("💬 {}", target_name)).size(24)).width(Length::Fill).center_x(),
                button("📞 Appeler").on_press(Message::CallContact(target_id.clone())).padding(10)
            ].align_items(Alignment::Center).width(Length::Fill);

            let mut chat_messages = column![].spacing(10);
            if let Some(history) = vd.chat_history.get(target_id) {
                for msg in history {
                    let is_me = msg.author == "Moi";
                    let color = if is_me { Color::from_rgb(0.2, 0.5, 0.8) } else { Color::from_rgb(0.8, 0.5, 0.2) };
                    let msg_text = text(format!("{}: {}", msg.author, msg.content)).size(16).style(color);
                    chat_messages = chat_messages.push(msg_text);
                }
            } else {
                chat_messages = chat_messages.push(text("Aucun message. Soyez le premier à écrire !").style(Color::from_rgb(0.6, 0.6, 0.6)));
            }

            let chat_scroll = Scrollable::new(chat_messages).height(Length::Fill).width(Length::Fill);

            let input_row = row![
                text_input(format!("Envoyer un message chiffré à {}...", target_name).as_str(), &self.chat_input)
                    .on_input(Message::ChatInputChanged)
                    .on_submit(Message::SendChatMessage)
                    .padding(10)
                    .width(Length::Fill),
                button("Envoyer").on_press(Message::SendChatMessage).padding(10)
            ].spacing(10);

            column![header, Rule::horizontal(10), chat_scroll, input_row].spacing(15).width(Length::Fill).height(Length::Fill)

        } else {
            column![
                text("💬 Bienvenue dans votre espace Zéro-Trace").size(28),
                text("Sélectionnez un contact à gauche pour afficher l'historique.").size(16),
                Rule::horizontal(20),
                text_input("Ajouter un ID public...", &self.peer_id_input).on_input(Message::PeerIdChanged).padding(10),
                button("Ajouter & Appeler").on_press(Message::ConnectClicked).padding(10),
                text(&self.status_message)
            ].spacing(20).align_items(Alignment::Center).width(Length::Fill)
        };

        let layout = row![
            Container::new(sidebar_scroll).padding(20),
            Rule::vertical(1),
            Container::new(main_content).padding(40).width(Length::Fill).height(Length::Fill)
        ];

        let content = column![layout, button("🔒 Verrouiller le coffre").on_press(Message::LockSession).padding(10)].align_items(Alignment::End);

        Container::new(content).width(Length::Fill).height(Length::Fill).into()
    }
}