use iced::widget::{button, column, row, text, text_input, Container, Scrollable, Rule, Space, container};
use iced::widget::scrollable::{Direction, Properties};
use iced::{alignment, Alignment, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{
    text_main, text_muted, color_for_user, dynamic_accent,
    PrimaryButton, SecondaryButton, SidebarButton, HangupButton, OverlayContainerStyle
};
use crate::ui::i18n::t;

struct BadgeStyle(iced::Color);
impl iced::widget::container::StyleSheet for BadgeStyle {
    type Style = iced::Theme;
    fn appearance(&self, _: &Self::Style) -> iced::widget::container::Appearance {
        iced::widget::container::Appearance {
            text_color: Some(iced::Color::WHITE),
            background: Some(iced::Background::Color(self.0)),
            border: iced::Border {
                radius: 10.0.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            shadow: iced::Shadow::default(),
        }
    }
}

impl KakolookiyamApp {
    pub(crate) fn view_group_overlay(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);
        let current_accent = dynamic_accent(&self.current_theme);

        let target_id = self.selected_chat.as_ref().unwrap();
        let vd = self.vault_data.as_ref().unwrap();
        let group_data = vd.groups.get(target_id).unwrap();
        let my_id = crate::crypto::derive_public_id(&vd.private_key);
        let is_creator = group_data.members.first() == Some(&my_id);

        let invite_row = row![
            text_input(&t(&self.language, "add_public_id"), &self.new_member_input)
                .on_input(Message::NewMemberInputChanged)
                .width(Length::Fill)
                .padding(10),
            button(text(t(&self.language, "btn_add")))
                .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                .on_press(Message::AddMemberToGroup)
                .padding(10)
        ].spacing(10);

        let mut quick_add_col = column![
            text(t(&self.language, "quick_add_contacts")).style(current_accent).size(18)
        ].spacing(10);

        let mut has_shortcuts = false;

        for (c_id, c_pseudo) in &vd.contacts {
            if !group_data.members.contains(c_id) {
                quick_add_col = quick_add_col.push(
                    button(text(format!("{} {}", t(&self.language, "btn_add_prefix"), c_pseudo)).size(16))
                        .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                        .on_press(Message::AddSpecificMemberToGroup(c_id.clone()))
                        .padding(10)
                        .width(Length::Fill)
                );
                has_shortcuts = true;
            }
        }

        if !has_shortcuts {
            quick_add_col = quick_add_col.push(
                text(t(&self.language, "all_contacts_in_server")).style(current_muted)
            );
        }

        let btn_delete_text = if is_creator { t(&self.language, "btn_delete_server") } else { t(&self.language, "btn_leave_server") };
        let btn_delete = button(text(btn_delete_text))
            .style(iced::theme::Button::Custom(Box::new(HangupButton)))
            .on_press(Message::DeleteGroup)
            .padding(15)
            .width(Length::Fill);

        let content = column![
            row![
                text(format!("{} {}", t(&self.language, "options_prefix"), group_data.name)).size(28).style(current_text),
                Space::with_width(Length::Fill),
                button(text(t(&self.language, "btn_close")))
                    .style(iced::theme::Button::Custom(Box::new(HangupButton)))
                    .on_press(Message::CloseGroupOptions)
                    .padding(10)
            ].align_items(Alignment::Center),

            Rule::horizontal(1),

            text(t(&self.language, "invite_member")).size(20).style(current_accent),
            invite_row,

            Space::with_height(10),

            Scrollable::new(quick_add_col)
                .height(Length::Fixed(150.0))
                .direction(Direction::Vertical(Properties::new().width(0).scroller_width(0))),

            Space::with_height(20),
            Rule::horizontal(1),
            btn_delete
        ]
        .spacing(20)
        .padding(40);

        let modal_box = Container::new(content)
            .width(Length::Fixed(640.0))
            .style(iced::theme::Container::Custom(Box::new(OverlayContainerStyle)));

        Container::new(modal_box).width(Length::Fill).height(Length::Fill).center_x().center_y().into()
    }

    pub(crate) fn view_dashboard(&self) -> Element<'_, Message> {
        if self.show_settings {
            return self.view_settings();
        }

        let vd = self.vault_data.as_ref().unwrap();
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);

        let badge_color = match format!("{:?}", self.current_theme).as_str() {
            "Dark" => iced::Color::from_rgb(0.9, 0.1, 0.1),
            _ => iced::Color::from_rgb(0.0, 0.5, 0.5),
        };

        let mut sidebar = column![
            text(format!("{}", vd.pseudo)).size(24).style(current_text),
            row![
                text(t(&self.language, "my_id")).size(14).style(current_muted),
                button(text(t(&self.language, "btn_copy")))
                    .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                    .on_press(Message::CopyIdClicked)
                    .padding(5)
            ]
            .spacing(10)
            .align_items(Alignment::Center),
            Rule::horizontal(1),
        ]
        .spacing(15)
        .width(Length::Fill);

        sidebar = sidebar.push(text(t(&self.language, "contacts")).size(16).style(current_muted));

        for (id, pseudo) in &vd.contacts {
            let is_selected = self.selected_chat.as_ref() == Some(id) && !self.show_group_options;
            let unread = self.unread_counts.get(id).copied().unwrap_or(0);

            let btn_content = if unread > 0 {
                row![
                    text(pseudo).size(16),
                    Space::with_width(Length::Fill),
                    container(text(unread.to_string()).size(12).style(iced::Color::WHITE))
                        .padding([2, 6])
                        .style(iced::theme::Container::Custom(Box::new(BadgeStyle(badge_color))))
                ].align_items(Alignment::Center)
            } else {
                row![text(pseudo).size(16)].align_items(Alignment::Center)
            };

            let btn_chat = button(btn_content)
                .style(iced::theme::Button::Custom(Box::new(SidebarButton { is_selected })))
                .on_press(Message::SelectChat(id.clone()))
                .width(Length::Fill);

            sidebar = sidebar.push(btn_chat);
        }

        sidebar = sidebar.push(Space::with_height(10));
        sidebar = sidebar.push(Rule::horizontal(1));
        sidebar = sidebar.push(text(t(&self.language, "servers")).size(16).style(current_muted));

        for (id, group) in &vd.groups {
            let is_selected = self.selected_chat.as_ref() == Some(id) && !self.show_group_options;
            let unread = self.unread_counts.get(id).copied().unwrap_or(0);

            let btn_content = if unread > 0 {
                row![
                    text(format!("# {}", group.name)).size(16),
                    Space::with_width(Length::Fill),
                    container(text(unread.to_string()).size(12).style(iced::Color::WHITE))
                        .padding([2, 6])
                        .style(iced::theme::Container::Custom(Box::new(BadgeStyle(badge_color))))
                ].align_items(Alignment::Center)
            } else {
                row![text(format!("# {}", group.name)).size(16)].align_items(Alignment::Center)
            };

            let btn_chat = button(btn_content)
                .style(iced::theme::Button::Custom(Box::new(SidebarButton { is_selected })))
                .on_press(Message::SelectChat(id.clone()))
                .width(Length::Fill);

            sidebar = sidebar.push(btn_chat);
        }

        let create_group_row = row![
            text_input(&t(&self.language, "new_server"), &self.new_group_input)
                .on_input(Message::NewGroupInputChanged)
                .width(Length::Fill),
            button(text("+"))
                .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                .on_press(Message::CreateGroup)
        ].spacing(5);

        sidebar = sidebar.push(Space::with_height(10));
        sidebar = sidebar.push(create_group_row);

        let sidebar_scroll = Scrollable::new(sidebar)
            .height(Length::Fill)
            .direction(Direction::Vertical(Properties::new().width(0).scroller_width(0)));

        let sidebar_content = column![
            sidebar_scroll,
            Rule::horizontal(1),
            button(text(t(&self.language, "btn_settings")))
                .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                .on_press(Message::OpenSettings)
                .padding(10)
                .width(Length::Fill)
        ]
        .width(Length::Fixed(280.0))
        .height(Length::Fill);

        let main_content: Element<'_, Message> = if let Some(target_id) = &self.selected_chat {
            if self.show_group_options {
                self.view_group_overlay()
            } else {
                let is_group = target_id.starts_with("grp_");
                let target_name = if let Some(p) = vd.contacts.get(target_id) {
                    p.clone()
                } else if let Some(g) = vd.groups.get(target_id) {
                    g.name.clone()
                } else {
                    "Inconnu".to_string()
                };

                let mut header = row![
                    button(text(t(&self.language, "btn_return")))
                        .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                        .on_press(Message::DeselectChat)
                        .padding(10),
                    Container::new(text(target_name.clone()).size(24).style(current_text))
                        .width(Length::Fill)
                        .center_x()
                ]
                .align_items(Alignment::Center)
                .width(Length::Fill);

                if is_group {
                    let btn_call_group = button(text(t(&self.language, "btn_call_group")))
                        .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                        .on_press(Message::CallContact(target_id.clone()))
                        .padding(10);

                    let btn_options = button(text(t(&self.language, "btn_options")))
                        .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                        .on_press(Message::OpenGroupOptions)
                        .padding(10);

                    header = header.push(
                        row![btn_call_group, btn_options].spacing(15).align_items(Alignment::Center)
                    );
                } else {
                    header = header.push(
                        row![
                            button(text(t(&self.language, "btn_copy_id")))
                                .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                                .on_press(Message::CopyContactId(target_id.clone()))
                                .padding(10),
                            button(text(t(&self.language, "btn_call_group")))
                                .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                                .on_press(Message::CallContact(target_id.clone()))
                                .padding(10)
                        ].spacing(10)
                    );
                }

                if let Some(handle) = &self.media_preview {
                    let img = iced::widget::image(handle.clone()).width(Length::Fill).height(Length::Fill);
                    let close_btn = button(text(t(&self.language, "btn_close_preview")))
                        .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                        .on_press(Message::ClosePreview)
                        .padding(15);

                    column![
                        header,
                        Rule::horizontal(1),
                        close_btn,
                        Container::new(img).center_x().center_y().width(Length::Fill).height(Length::Fill)
                    ]
                    .spacing(15)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
                } else {
                    let mut chat_messages = column![].spacing(15);

                    if let Some(history) = vd.chat_history.get(target_id) {
                        for msg in history {
                            let color = color_for_user(&msg.author, &self.current_theme);

                            if msg.is_media {
                                let key = msg.media_key.unwrap_or([0u8; 32]);
                                let path = msg.media_path.clone().unwrap_or_default();
                                let is_image = msg.content.to_lowercase().contains(".png")
                                            || msg.content.to_lowercase().contains(".jpg");

                                let text_element = text(format!("{}: {}", msg.author, msg.content))
                                    .size(16)
                                    .style(color)
                                    .width(Length::Fill);

                                let mut btn_row = row![
                                    button(text(t(&self.language, "btn_extract")))
                                        .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                                        .on_press(Message::OpenMedia(msg.content.clone(), key, path.clone()))
                                        .padding(8)
                                ].spacing(10);

                                if is_image {
                                    btn_row = btn_row.push(
                                        button(text(t(&self.language, "btn_preview_ram")))
                                            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                                            .on_press(Message::PreviewMedia(path, key))
                                            .padding(8)
                                    );
                                }

                                let media_row = row![text_element, btn_row]
                                    .spacing(15)
                                    .align_items(Alignment::Center)
                                    .width(Length::Fill);

                                chat_messages = chat_messages.push(media_row);
                            } else {
                                chat_messages = chat_messages.push(
                                    text(format!("{}: {}", msg.author, msg.content))
                                        .size(16)
                                        .style(color)
                                        .width(Length::Fill)
                                );
                            }
                        }
                    } else {
                        chat_messages = chat_messages.push(
                            text(t(&self.language, "no_message")).style(current_muted)
                        );
                    }

                    let chat_scroll = Scrollable::new(chat_messages)
                        .height(Length::Fill)
                        .width(Length::Fill)
                        .direction(Direction::Vertical(Properties::new().width(0).scroller_width(0)));

                    let input_row = row![
                        button(text("📎"))
                            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                            .on_press(Message::OpenFileDialog)
                            .padding(10),
                        text_input(&format!("{} {}...", t(&self.language, "send_to_prefix"), target_name), &self.chat_input)
                            .on_input(Message::ChatInputChanged)
                            .on_submit(Message::SendChatMessage)
                            .padding(10)
                            .width(Length::Fill),
                        button(text(t(&self.language, "btn_send")))
                            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                            .on_press(Message::SendChatMessage)
                            .padding(10)
                    ].spacing(10);

                    let mut chat_column = column![header, Rule::horizontal(1), chat_scroll]
                        .spacing(15)
                        .width(Length::Fill)
                        .height(Length::Fill);

                    if !self.status_message.is_empty() {
                        let alert_color = if self.status_message.starts_with("ERROR:") {
                            iced::Color::from_rgb(0.9, 0.1, 0.1)
                        } else {
                            dynamic_accent(&self.current_theme)
                        };
                        chat_column = chat_column.push(text(&self.status_message).style(alert_color).size(14));
                    }

                    chat_column.push(input_row).into()
                }
            }
        } else {
            column![
                text(t(&self.language, "welcome_chat_1"))
                    .size(28)
                    .style(current_text)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center),
                text(t(&self.language, "welcome_chat_2"))
                    .size(28)
                    .style(current_text)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center),
                text(t(&self.language, "select_contact"))
                    .size(16)
                    .style(current_muted)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center),
                Rule::horizontal(1),
                text_input(&t(&self.language, "add_public_id_call"), &self.peer_id_input)
                    .on_input(Message::PeerIdChanged)
                    .padding(10),
                button(text(t(&self.language, "btn_add_call")))
                    .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
                    .on_press(Message::ConnectClicked)
                    .padding(10),
                text(&self.status_message)
                    .style(current_text)
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
            ]
            .spacing(20)
            .align_items(Alignment::Center)
            .width(Length::Fill)
            .into()
        };

        let layout = row![
            Container::new(sidebar_content).padding(20),
            Rule::vertical(1),
            Container::new(main_content).padding(40).width(Length::Fill).height(Length::Fill)
        ]
        .width(Length::Fill)
        .height(Length::Fill);

        let bottom_bar = Container::new(
            button(text(t(&self.language, "btn_lock")))
                .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                .on_press(Message::LockSession)
                .padding(10)
        )
        .width(Length::Fill)
        .align_x(alignment::Horizontal::Right);

        let content = column![layout, bottom_bar].width(Length::Fill).height(Length::Fill);

        Container::new(content).width(Length::Fill).height(Length::Fill).into()
    }
}