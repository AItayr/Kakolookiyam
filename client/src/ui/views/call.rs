use crate::ui::app::KakolookiyamApp;
use crate::ui::i18n::t;
use crate::ui::messages::Message;
use crate::ui::theme::{
    color_for_user, hangup_button, primary_button, secondary_button, text_main, text_muted,
};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Container, Scrollable, button, column, row, text_input};
use iced::{Alignment, Element, Length};

impl KakolookiyamApp {
    #[allow(dead_code)]
    pub(crate) fn view_active_call(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);

        let (_, active_pseudo) = self.active_call.as_ref().unwrap();

        let title = crate::ui::i18n::app_text(&self.language, t(&self.language, "call_active"))
            .size(40)
            .color(current_text);

        let subtitle = crate::ui::i18n::app_text(
            &self.language,
            format!(
                "{} {}",
                t(&self.language, "call_secure_with"),
                active_pseudo
            ),
        )
        .size(25)
        .color(current_muted);

        let mute_text = if self.is_muted {
            t(&self.language, "mic_enable")
        } else {
            t(&self.language, "mic_disable")
        };

        let btn_mute = button(crate::ui::i18n::app_text(&self.language, mute_text))
            .style(
                secondary_button
                    as fn(
                        &iced::Theme,
                        iced::widget::button::Status,
                    ) -> iced::widget::button::Style,
            )
            .on_press(Message::ToggleMute)
            .padding(20);

        let btn_hangup = button(crate::ui::i18n::app_text(
            &self.language,
            t(&self.language, "btn_hangup"),
        ))
        .style(
            hangup_button
                as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style,
        )
        .on_press(Message::HangUpCall)
        .padding(20);

        let buttons = row![btn_mute, btn_hangup].spacing(40);

        let mut chat_messages = column![].spacing(10);
        for (author, msg) in &self.chat_history {
            let msg_text =
                crate::ui::i18n::app_text(&self.language, format!("{}: {}", author, msg))
                    .size(16)
                    .color(color_for_user(author, &self.current_theme));
            chat_messages = chat_messages.push(msg_text);
        }

        let chat_scroll = Scrollable::new(chat_messages)
            .height(Length::Fixed(250.0))
            .width(Length::Fixed(600.0))
            .direction(Direction::Vertical(
                Scrollbar::new().width(0).scroller_width(0),
            ));

        let input_chat = text_input(
            &t(&self.language, "chat_placeholder_stealth"),
            &self.chat_input,
        )
        .on_input(Message::ChatInputChanged)
        .on_submit(Message::SendChatMessage)
        .padding(10);

        let btn_send = button(crate::ui::i18n::app_text(
            &self.language,
            t(&self.language, "btn_send"),
        ))
        .style(
            primary_button
                as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style,
        )
        .on_press(Message::SendChatMessage)
        .padding(10);

        let input_row = row![input_chat, btn_send]
            .spacing(10)
            .width(Length::Fixed(600.0));

        let chat_section = column![
            crate::ui::i18n::app_text(&self.language, t(&self.language, "chat_ephemeral"))
                .size(20)
                .color(current_text),
            chat_scroll,
            input_row
        ]
        .spacing(15)
        .align_x(Alignment::Center);

        let content = column![title, subtitle, buttons, chat_section]
            .spacing(40)
            .align_x(Alignment::Center);

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(iced::Length::Fill)
            .center_y(iced::Length::Fill)
            .into()
    }

    pub(crate) fn view_unlocked(&self) -> Element<'_, Message> {
        // [MED-4 UX] On ne bloque plus la vue complte lorsqu'un appel est actif. Le bandeau d'appel
        // est directement affich tout en haut du dashboard ! (cf view_dashboard)
        self.view_dashboard()
    }
}
