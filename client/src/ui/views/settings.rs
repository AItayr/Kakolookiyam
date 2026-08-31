use iced::widget::{button, column, row, text, Container, Rule, Space, pick_list, Scrollable};
use iced::widget::scrollable::{Direction, Properties};
use iced::{Alignment, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{text_main, text_muted, dynamic_accent, SecondaryButton, HangupButton, OverlayContainerStyle};
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
                    text(legal_title).size(28).style(current_text),
                    Space::with_width(Length::Fill),
                    button(text(t(&self.language, "btn_close")))
                        .style(iced::theme::Button::Custom(Box::new(HangupButton)))
                        .on_press(Message::ToggleLegal(tab.clone()))
                        .padding(10)
                ].align_items(Alignment::Center),

                Rule::horizontal(1),

                Scrollable::new(text(legal_text).style(current_text).size(16))
                    .height(Length::Fill)
                    .direction(Direction::Vertical(Properties::new().width(0).scroller_width(0)))
            ]
            .spacing(20)
            .padding(40);

            let modal_box = Container::new(content)
                .width(Length::Fixed(640.0))
                .height(Length::Fixed(480.0))
                .style(iced::theme::Container::Custom(Box::new(OverlayContainerStyle)));

            return Container::new(modal_box).width(Length::Fill).height(Length::Fill).center_x().center_y().into();
        }

        // --- VUE NORMALE DES PARAMÈTRES ---
        let mics = vec![
            t(&self.language, "device_default"),
            "Universal Audio Volt 476".to_string(),
            t(&self.language, "device_virtual_in")
        ];
        let spks = vec![
            t(&self.language, "device_default"),
            "Universal Audio Volt 476".to_string(),
            t(&self.language, "device_virtual_out")
        ];

        let mic_picker = pick_list(mics, Some(self.selected_mic.clone()), Message::MicSelected).width(Length::Fill);
        let spk_picker = pick_list(spks, Some(self.selected_speaker.clone()), Message::SpeakerSelected).width(Length::Fill);

        let legal_section = column![
            text(t(&self.language, "settings_legal")).size(20).style(current_accent),
            row![
                button(text(t(&self.language, "settings_cgu_btn")))
                    .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                    .on_press(Message::ToggleLegal("CGU".to_string())).padding(10),
                button(text(t(&self.language, "privacy_title")))
                    .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                    .on_press(Message::ToggleLegal("PRIVACY".to_string())).padding(10),
                button(text(t(&self.language, "ofl_title")))
                    .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                    .on_press(Message::ToggleLegal("OFL".to_string())).padding(10),
            ].spacing(15)
        ].spacing(10);

        let settings_content = column![
            row![
                text(t(&self.language, "settings_title")).size(32).style(current_text),
                Space::with_width(Length::Fill),
                button(text(t(&self.language, "btn_close")))
                    .style(iced::theme::Button::Custom(Box::new(HangupButton)))
                    .on_press(Message::CloseSettings)
                    .padding(10)
            ].align_items(Alignment::Center).width(Length::Fill),

            Rule::horizontal(1),

            text(t(&self.language, "settings_general")).size(20).style(current_accent),
            button(text(t(&self.language, "settings_lang")))
                .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                .on_press(Message::ToggleLanguage)
                .padding(10),

            button(text(t(&self.language, "settings_theme")))
                .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
                .on_press(Message::ToggleTheme)
                .padding(10),


            Space::with_height(20),

            text(t(&self.language, "settings_audio")).size(20).style(current_accent),
            text(t(&self.language, "settings_mic")).style(current_muted),
            mic_picker,
            Space::with_height(5),
            text(t(&self.language, "settings_speaker")).style(current_muted),
            spk_picker,

            Space::with_height(20),
            legal_section,
        ]
        .spacing(20)
        .padding(60)
        .width(Length::Fill);

        let settings_scroll = Scrollable::new(settings_content)
            .height(Length::Fill)
            .direction(Direction::Vertical(Properties::new().width(0).scroller_width(0)));

        Container::new(settings_scroll).width(Length::Fill).height(Length::Fill).into()
    }
}