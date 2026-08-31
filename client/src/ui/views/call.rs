use iced::widget::{button, column, row, text, text_input, Container, Scrollable};
use iced::widget::scrollable::{Direction, Properties};
use iced::{Alignment, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{text_main, text_muted, color_for_user, PrimaryButton, SecondaryButton, HangupButton};
use crate::ui::i18n::t;

impl KakolookiyamApp {
    pub(crate) fn view_active_call(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);

        let (_, active_pseudo) = self.active_call.as_ref().unwrap();

        let title = text(t(&self.language, "call_active"))
            .size(40)
            .style(current_text);

        let subtitle = text(format!("{} {}", t(&self.language, "call_secure_with"), active_pseudo))
            .size(25)
            .style(current_muted);

        let mute_text = if self.is_muted { t(&self.language, "mic_enable") } else { t(&self.language, "mic_disable") };

        let btn_mute = button(text(mute_text))
            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
            .on_press(Message::ToggleMute)
            .padding(20);

        let btn_hangup = button(text(t(&self.language, "btn_hangup")))
            .style(iced::theme::Button::Custom(Box::new(HangupButton)))
            .on_press(Message::HangUpCall)
            .padding(20);

        let buttons = row![btn_mute, btn_hangup].spacing(40);

        let mut chat_messages = column![].spacing(10);
        for (author, msg) in &self.chat_history {
            let msg_text = text(format!("{}: {}", author, msg))
                .size(16)
                .style(color_for_user(author, &self.current_theme));
            chat_messages = chat_messages.push(msg_text);
        }

        let chat_scroll = Scrollable::new(chat_messages)
            .height(Length::Fixed(250.0))
            .width(Length::Fixed(600.0))
            .direction(Direction::Vertical(Properties::new().width(0).scroller_width(0)));

        let input_chat = text_input(&t(&self.language, "chat_placeholder_stealth"), &self.chat_input)
            .on_input(Message::ChatInputChanged)
            .on_submit(Message::SendChatMessage)
            .padding(10);

        let btn_send = button(text(t(&self.language, "btn_send")))
            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
            .on_press(Message::SendChatMessage)
            .padding(10);

        let input_row = row![input_chat, btn_send]
            .spacing(10)
            .width(Length::Fixed(600.0));

        let chat_section = column![
            text(t(&self.language, "chat_ephemeral")).size(20).style(current_text),
            chat_scroll,
            input_row
        ]
        .spacing(15)
        .align_items(Alignment::Center);

        let content = column![title, subtitle, buttons, chat_section]
            .spacing(40)
            .align_items(Alignment::Center);

        Container::new(content).width(Length::Fill).height(Length::Fill).center_x().center_y().into()
    }

    pub(crate) fn view_incoming_call(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);

        let (caller_id, caller_pseudo, sdp) = self.incoming_call.as_ref().unwrap();

        let title = text(t(&self.language, "call_incoming"))
            .size(40)
            .style(current_text);

        let subtitle = text(format!("{} {}", caller_pseudo, t(&self.language, "wants_to_talk")))
            .size(25)
            .style(current_muted);

        let timer_text = text(format!("({} {}s)", t(&self.language, "auto_reject"), 15 - self.incoming_call_timer))
            .size(16)
            .style(crate::ui::theme::ACCENT_RED_DARK);

        let btn_accept = button(text(t(&self.language, "btn_accept")))
            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
            .on_press(Message::AcceptCall(caller_id.clone(), sdp.clone()))
            .padding(15);

        let btn_reject = button(text(t(&self.language, "btn_reject")))
            .style(iced::theme::Button::Custom(Box::new(HangupButton)))
            .on_press(Message::RejectCall(caller_id.clone()))
            .padding(15);

        let buttons = row![btn_reject, btn_accept].spacing(30);

        let content = column![title, subtitle, timer_text, buttons]
            .spacing(25)
            .padding(60)
            .align_items(Alignment::Center);

        Container::new(content).width(Length::Fill).height(Length::Fill).center_x().center_y().into()
    }

    pub(crate) fn view_unlocked(&self) -> Element<'_, Message> {
        if self.active_call.is_some() { return self.view_active_call(); }
        if self.incoming_call.is_some() { return self.view_incoming_call(); }
        self.view_dashboard()
    }
}