use iced::widget::{button, column, row, text_input, Container, Scrollable};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Alignment, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{text_main, text_muted, color_for_user, primary_button, secondary_button, hangup_button};
use crate::ui::i18n::t;

impl KakolookiyamApp {
    pub(crate) fn view_active_call(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);

        let (_, active_pseudo) = self.active_call.as_ref().unwrap();

        let title = crate::ui::i18n::app_text(&self.language, t(&self.language, "call_active"))
            .size(40)
            .color(current_text);

        let subtitle = crate::ui::i18n::app_text(&self.language, format!("{} {}", t(&self.language, "call_secure_with"), active_pseudo))
            .size(25)
            .color(current_muted);

        let mute_text = if self.is_muted { t(&self.language, "mic_enable") } else { t(&self.language, "mic_disable") };

        let btn_mute = button(crate::ui::i18n::app_text(&self.language, mute_text))
            .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::ToggleMute)
            .padding(20);

        let btn_hangup = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_hangup")))
            .style(hangup_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::HangUpCall)
            .padding(20);

        let buttons = row![btn_mute, btn_hangup].spacing(40);

        let mut chat_messages = column![].spacing(10);
        for (author, msg) in &self.chat_history {
            let msg_text = crate::ui::i18n::app_text(&self.language, format!("{}: {}", author, msg))
                .size(16)
                .color(color_for_user(author, &self.current_theme));
            chat_messages = chat_messages.push(msg_text);
        }

        let chat_scroll = Scrollable::new(chat_messages)
            .height(Length::Fixed(250.0))
            .width(Length::Fixed(600.0))
            .direction(Direction::Vertical(Scrollbar::new().width(0).scroller_width(0)));

        let input_chat = text_input(&t(&self.language, "chat_placeholder_stealth"), &self.chat_input)
            .on_input(Message::ChatInputChanged)
            .on_submit(Message::SendChatMessage)
            .padding(10);

        let btn_send = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_send")))
            .style(primary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::SendChatMessage)
            .padding(10);

        let input_row = row![input_chat, btn_send]
            .spacing(10)
            .width(Length::Fixed(600.0));

        let chat_section = column![
            crate::ui::i18n::app_text(&self.language, t(&self.language, "chat_ephemeral")).size(20).color(current_text),
            chat_scroll,
            input_row
        ]
        .spacing(15)
        .align_x(Alignment::Center);

        let content = column![title, subtitle, buttons, chat_section]
            .spacing(40)
            .align_x(Alignment::Center);

        Container::new(content).width(Length::Fill).height(Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill).into()
    }

    pub(crate) fn view_incoming_call(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);

        let (caller_id, caller_pseudo, sdp) = self.incoming_call.as_ref().unwrap();

        let title = crate::ui::i18n::app_text(&self.language, t(&self.language, "call_incoming"))
            .size(40)
            .color(current_text);

        let subtitle = crate::ui::i18n::app_text(&self.language, format!("{} {}", caller_pseudo, t(&self.language, "wants_to_talk")))
            .size(25)
            .color(current_muted);

        let timer_text = crate::ui::i18n::app_text(&self.language, format!("({} {}s)", t(&self.language, "auto_reject"), 15 - self.incoming_call_timer))
            .size(16)
            .color(crate::ui::theme::ACCENT_RED_DARK);

        let btn_accept = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_accept")))
            .style(primary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::AcceptCall(caller_id.clone(), sdp.clone()))
            .padding(15);

        let btn_reject = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_reject")))
            .style(hangup_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::RejectCall(caller_id.clone()))
            .padding(15);

        let buttons = row![btn_reject, btn_accept].spacing(30);

        let content = column![title, subtitle, timer_text, buttons]
            .spacing(25)
            .padding(60)
            .align_x(Alignment::Center);

        Container::new(content).width(Length::Fill).height(Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill).into()
    }

    pub(crate) fn view_loading_screen(&self) -> Element<'_, Message> {
        let current_text = crate::ui::theme::text_main(&self.current_theme);
        let current_accent = crate::ui::theme::dynamic_accent(&self.current_theme);
        
        let msg = self.status_message.trim_start_matches("LOADING:");
        
        let title = crate::ui::i18n::app_text(&self.language, "Sécurisation de la connexion")
            .size(36)
            .color(current_accent);
            
        let subtitle = crate::ui::i18n::app_text(&self.language, msg)
            .size(24)
            .color(current_text);

        let content = iced::widget::column![title, subtitle]
            .spacing(30)
            .align_x(iced::Alignment::Center);

        iced::widget::Container::new(content).width(iced::Length::Fill).height(iced::Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill).into()
    }

    pub(crate) fn view_unlocked(&self) -> Element<'_, Message> {
        if self.active_call.is_some() { return self.view_active_call(); }
        if self.incoming_call.is_some() { return self.view_incoming_call(); }
        if self.status_message.starts_with("LOADING:") { return self.view_loading_screen(); }
        self.view_dashboard()
    }
}