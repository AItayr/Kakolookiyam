use iced::widget::{button, column, row, Container, Space, pick_list, Scrollable};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::{Alignment, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{text_main, text_muted, dynamic_accent, secondary_button, hangup_button, overlay_container_style};
use crate::ui::i18n::t;

impl KakolookiyamApp {
    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let current_text = text_main(&self.current_theme);
        let current_muted = text_muted(&self.current_theme);
        let current_accent = dynamic_accent(&self.current_theme);

        // --- GESTION DE L'OVERLAY LÉGAL ---
        if let Some(tab) = &self.active_legal_tab {
            let (legal_title, legal_text) = match tab.as_str() {
                "CGU" => (t(&self.language, "cgu_title"), t(&self.language, "cgu_text")),
                "PRIVACY" => (t(&self.language, "privacy_title"), t(&self.language, "privacy_text")),
                "OFL" => (t(&self.language, "ofl_title"), t(&self.language, "ofl_text")),
                _ => (String::new(), String::new())
            };

            let content = column![
                row![
                    crate::ui::i18n::app_text(&self.language, legal_title).size(28).color(current_text),
                    Space::new().width(Length::Fill),
                    button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_close")))
                        .style(hangup_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                        .on_press(Message::ToggleLegal(tab.clone()))
                        .padding(10)
                ].align_y(Alignment::Center),

                iced::widget::rule::horizontal(1),

                Scrollable::new(crate::ui::i18n::app_text(&self.language, legal_text).color(current_text).size(16))
                    .height(Length::Fill)
                    .direction(Direction::Vertical(Scrollbar::new().width(0).scroller_width(0)))
            ]
            .spacing(20)
            .padding(40);

            let modal_box = Container::new(content)
                .width(Length::Fixed(640.0))
                .height(Length::Fixed(480.0))
                .style(overlay_container_style as fn(&iced::Theme) -> iced::widget::container::Style);

            return Container::new(modal_box).width(Length::Fill).height(Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill).into();
        }

        // --- VUE NORMALE DES PARAMÈTRES ---
        let mics = self.available_mics.clone();
        let spks = self.available_speakers.clone();

        let mic_picker = pick_list(mics, Some(self.selected_mic.clone()), Message::MicSelected).width(Length::Fill);
        let spk_picker = pick_list(spks, Some(self.selected_speaker.clone()), Message::SpeakerSelected).width(Length::Fill);

        let legal_section = column![
            crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_legal")).size(20).color(current_accent),
            row![
                button(crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_cgu_btn")))
                    .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                    .on_press(Message::ToggleLegal("CGU".to_string())).padding(10),
                button(crate::ui::i18n::app_text(&self.language, t(&self.language, "privacy_title")))
                    .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                    .on_press(Message::ToggleLegal("PRIVACY".to_string())).padding(10),
                button(crate::ui::i18n::app_text(&self.language, t(&self.language, "ofl_title")))
                    .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                    .on_press(Message::ToggleLegal("OFL".to_string())).padding(10),
            ].spacing(15)
        ].spacing(10);

        let settings_content = column![
            row![
                crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_title")).size(32).color(current_text),
                Space::new().width(Length::Fill),
                button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_close")))
                    .style(hangup_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                    .on_press(Message::CloseSettings)
                    .padding(10)
            ].align_y(Alignment::Center).width(Length::Fill),

            iced::widget::rule::horizontal(1),

            crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_general")).size(20).color(current_accent),
            button(crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_lang")))
                .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                .on_press(Message::ToggleLanguage)
                .padding(10),

            button(crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_theme")))
                .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
                .on_press(Message::ToggleTheme)
                .padding(10),


            Space::new().height(20),

            crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_audio")).size(20).color(current_accent),
            crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_mic")).color(current_muted),
            mic_picker,
            Space::new().height(5),
            crate::ui::i18n::app_text(&self.language, t(&self.language, "settings_speaker")).color(current_muted),
            spk_picker,

            Space::new().height(20),
            legal_section,
        ]
        .spacing(20)
        .padding(60)
        .width(Length::Fill);

        let settings_scroll = Scrollable::new(settings_content)
            .height(Length::Fill)
            .direction(Direction::Vertical(Scrollbar::new().width(0).scroller_width(0)));

        Container::new(settings_scroll).width(Length::Fill).height(Length::Fill).into()
    }
}