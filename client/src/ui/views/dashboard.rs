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

        let create_group_row = row![
            text_input("Nouveau serveur...", &self.new_group_input).on_input(Message::NewGroupInputChanged).width(Length::Fill),
            button("+").on_press(Message::CreateGroup)
        ].spacing(5);
        sidebar = sidebar.push(create_group_row);

        let sidebar_scroll = Scrollable::new(sidebar).height(Length::Fill);

        let main_content = if let Some(target_id) = &self.selected_chat {
            let is_group = target_id.starts_with("grp_");
            let target_name = if let Some(p) = vd.contacts.get(target_id) { p.clone() }
                              else if let Some(g) = vd.groups.get(target_id) { g.name.clone() }
                              else { "Inconnu".to_string() };

            let mut header = row![
                button("⬅️ Retour").on_press(Message::DeselectChat).padding(10),
                Container::new(text(format!("💬 {}", target_name)).size(24)).width(Length::Fill).center_x()
            ].align_items(Alignment::Center).width(Length::Fill);

            if is_group {
                if let Some(group_data) = vd.groups.get(target_id) {
                    let my_id = crate::crypto::derive_public_id(&vd.private_key);
                    let is_creator = group_data.members.first() == Some(&my_id);

                    let invite_row = row![
                        text_input("ID à inviter...", &self.new_member_input).on_input(Message::NewMemberInputChanged).width(Length::Fixed(150.0)),
                        button("Ajouter").on_press(Message::AddMemberToGroup)
                    ].spacing(5);

                    let mut quick_add_row = row![].spacing(5).align_items(Alignment::Center);
                    let mut has_shortcuts = false;
                    for (c_id, c_pseudo) in &vd.contacts {
                        if !group_data.members.contains(c_id) {
                            quick_add_row = quick_add_row.push(
                                button(text(format!("+ {}", c_pseudo)).size(12))
                                    .on_press(Message::AddSpecificMemberToGroup(c_id.clone()))
                                    .padding(4)
                            );
                            has_shortcuts = true;
                        }
                    }

                    let mut group_controls = column![invite_row].spacing(5).align_items(Alignment::End);
                    if has_shortcuts {
                        group_controls = group_controls.push(quick_add_row);
                    }

                    let btn_call_group = button("📞 Appeler le serveur")
                        .on_press(Message::CallContact(target_id.clone()))
                        .padding(10);

                    let btn_delete = button(if is_creator { "🗑️ Supprimer" } else { "🚪 Quitter" })
                        .on_press(Message::DeleteGroup)
                        .padding(10);

                    header = header.push(row![group_controls, btn_call_group, btn_delete].spacing(15).align_items(Alignment::Center));
                }
            } else {
                header = header.push(row![
                    button("📋 Copier son ID").on_press(Message::CopyContactId(target_id.clone())).padding(10),
                    button("📞 Appeler").on_press(Message::CallContact(target_id.clone())).padding(10)
                ].spacing(10));
            }

            // --- ÉTAPE 1 : Rendu de l'image SI APERÇU ACTIF ---
            if let Some(handle) = &self.media_preview {
                let img = iced::widget::image(handle.clone())
                    .width(Length::Fill)
                    .height(Length::Fill);

                let close_btn = button("Fermer l'aperçu (Purger la RAM)")
                    .on_press(Message::ClosePreview)
                    .padding(15);

                column![
                    header,
                    Rule::horizontal(10),
                    close_btn,
                    Container::new(img).center_x().center_y().width(Length::Fill).height(Length::Fill)
                ].spacing(15).width(Length::Fill).height(Length::Fill)

            } else {
                let mut chat_messages = column![].spacing(10);
                if let Some(history) = vd.chat_history.get(target_id) {
                    for msg in history {
                        let is_me = msg.author == "Moi";
                        let color = if is_me { Color::from_rgb(0.2, 0.5, 0.8) } else { Color::from_rgb(0.8, 0.5, 0.2) };

                        if msg.is_media {
                            let key = msg.media_key.unwrap_or([0u8; 32]);
                            let path = msg.media_path.clone().unwrap_or_default();

                            let is_image = msg.content.to_lowercase().contains(".png") || msg.content.to_lowercase().contains(".jpg");

                            let mut media_row = row![
                                text(format!("{}: {}", msg.author, msg.content)).size(16).style(color),
                                button("📂 Extraire & Ouvrir")
                                    .on_press(Message::OpenMedia(msg.content.clone(), key, path.clone()))
                                    .padding(5)
                            ].spacing(10).align_items(Alignment::Center);

                            // --- NOUVEAU BOUTON APERÇU ---
                            if is_image {
                                media_row = media_row.push(
                                    button("👁️ Aperçu Zéro-Trace")
                                    .on_press(Message::PreviewMedia(path, key))
                                    .padding(5)
                                );
                            }

                            chat_messages = chat_messages.push(media_row);
                        } else {
                            let msg_text = text(format!("{}: {}", msg.author, msg.content)).size(16).style(color);
                            chat_messages = chat_messages.push(msg_text);
                        }
                    }
                } else {
                    chat_messages = chat_messages.push(text("Aucun message. Soyez le premier à écrire !").style(Color::from_rgb(0.6, 0.6, 0.6)));
                }

                let chat_scroll = Scrollable::new(chat_messages).height(Length::Fill).width(Length::Fill);

                let input_row = row![
                    button("📎").on_press(Message::OpenFileDialog).padding(10),
                    text_input(format!("Envoyer à {}...", target_name).as_str(), &self.chat_input)
                        .on_input(Message::ChatInputChanged)
                        .on_submit(Message::SendChatMessage)
                        .padding(10)
                        .width(Length::Fill),
                    button("Envoyer").on_press(Message::SendChatMessage).padding(10)
                ].spacing(10);

                let mut chat_column = column![header, Rule::horizontal(10), chat_scroll]
                    .spacing(15)
                    .width(Length::Fill)
                    .height(Length::Fill);

                if !self.status_message.is_empty() {
                    let text_color = if self.status_message.starts_with('❌') {
                        Color::from_rgb(0.9, 0.1, 0.1)
                    } else if self.status_message.starts_with('✅') {
                        Color::from_rgb(0.1, 0.7, 0.1)
                    } else {
                        Color::from_rgb(0.5, 0.5, 0.5)
                    };
                    chat_column = chat_column.push(text(&self.status_message).style(text_color).size(14));
                }

                chat_column.push(input_row)
            }

        } else {
            column![
                text("💬 Bienvenue dans votre espace Zéro-Trace").size(28),
                text("Sélectionnez un contact ou un serveur à gauche.").size(16),
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