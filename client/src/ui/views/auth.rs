use iced::widget::{button, column, container, row, text_input, Container};
use iced::{alignment, Alignment, Color, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{text_main, text_muted, primary_button, secondary_button, error_container_style};
use crate::ui::i18n::t;

impl KakolookiyamApp {
    pub(crate) fn clear_auth_fields(&mut self) {
        self.pseudo_input.clear();
        self.password_input.clear();
        self.password_confirm_input.clear();
        self.auth_error = None;
    }

    pub(crate) fn view_welcome(&self) -> Element<'_, Message> {
        let lang_btn = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "lang_toggle")))
            .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::ToggleLanguage)
            .padding(10);
        let top_bar = container(lang_btn).width(Length::Fill).align_x(alignment::Horizontal::Right).padding(20);

        let title = crate::ui::i18n::app_text(&self.language, "Kakolookiyam")
            .size(45)
            .color(text_main(&self.current_theme));

        let subtitle = crate::ui::i18n::app_text(&self.language, t(&self.language, "welcome_sub"))
            .size(18)
            .color(text_muted(&self.current_theme));

        let btn_create = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_create")))
            .style(primary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::GoToCreateAccount)
            .padding(15);

        let btn_login = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_login")))
            .style(primary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::GoToLogin)
            .padding(15);

        let content = column![title, subtitle, btn_create, btn_login]
            .spacing(25)
            .padding(60)
            .align_x(Alignment::Center)
            .width(Length::Fill);

        let main_box = Container::new(content).width(Length::Fill).height(Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill);
        column![top_bar, main_box].width(Length::Fill).height(Length::Fill).into()
    }

    pub(crate) fn view_create_account(&self) -> Element<'_, Message> {
        let lang_btn = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "lang_toggle")))
            .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::ToggleLanguage)
            .padding(10);
        let top_bar = container(lang_btn).width(Length::Fill).align_x(alignment::Horizontal::Right).padding(20);

        let title = column![
            crate::ui::i18n::app_text(&self.language, "KAKOLOOKIYAM")
                .size(45)
                .color(text_main(&self.current_theme)),
            crate::ui::i18n::app_text(&self.language, t(&self.language, "create_vault_title"))
                .size(30)
                .color(text_main(&self.current_theme))
        ].spacing(10).align_x(Alignment::Center);

        let info = crate::ui::i18n::app_text(&self.language, t(&self.language, "create_vault_info"))
            .color(text_muted(&self.current_theme));

        let pseudo_input = text_input(&t(&self.language, "pseudo_placeholder"), &self.pseudo_input)
            .on_input(Message::PseudoChanged)
            .padding(15);

        let pass_input = text_input(
            &t(&self.language, "password_new_placeholder"),
            &self.password_input
        )
        .on_input(Message::PasswordChanged)
        .secure(true)
        .padding(15);

        let pass_confirm = text_input(&t(&self.language, "password_confirm_placeholder"), &self.password_confirm_input)
            .on_input(Message::PasswordConfirmChanged)
            .secure(true)
            .padding(15);

        let mut col = column![title, info, pseudo_input, pass_input, pass_confirm].spacing(20);

        if let Some(err) = &self.auth_error {
            let error_box = container(
                crate::ui::i18n::app_text(&self.language, format!("{} : {}", t(&self.language, "error"), err)).size(14).color(Color::from_rgb(0.9, 0.15, 0.15))
            )
            .padding(12)
            .width(Length::Fill)
            .style(error_container_style as fn(&iced::Theme) -> iced::widget::container::Style);

            col = col.push(error_box);
        }

        let btn_submit = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_submit_create")))
            .style(primary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::SubmitCreateAccount)
            .padding(12);

        let btn_back = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_back")))
            .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::BackToWelcome)
            .padding(12);

        col = col.push(row![btn_back, btn_submit].spacing(15)).align_x(Alignment::Center);

        let main_box = Container::new(col).width(Length::Fill).height(Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill);
        column![top_bar, main_box].width(Length::Fill).height(Length::Fill).into()
    }

    pub(crate) fn view_login(&self) -> Element<'_, Message> {
        let lang_btn = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "lang_toggle")))
            .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::ToggleLanguage)
            .padding(10);
        let top_bar = container(lang_btn).width(Length::Fill).align_x(alignment::Horizontal::Right).padding(20);

        let title = column![
            crate::ui::i18n::app_text(&self.language, "KAKOLOOKIYAM")
                .size(45)
                .color(text_main(&self.current_theme)),
            crate::ui::i18n::app_text(&self.language, t(&self.language, "login_title"))
                .size(30)
                .color(text_main(&self.current_theme))
        ].spacing(10).align_x(Alignment::Center);

        let pseudo_input = text_input(&t(&self.language, "login_pseudo_placeholder"), &self.pseudo_input)
            .on_input(Message::PseudoChanged)
            .padding(15);

        let pass_input = text_input(&t(&self.language, "login_password_placeholder"), &self.password_input)
            .on_input(Message::PasswordChanged)
            .secure(true)
            .padding(15);

        let mut col = column![title, pseudo_input, pass_input].spacing(20);

        if let Some(err) = &self.auth_error {
            let error_box = container(
                crate::ui::i18n::app_text(&self.language, format!("{} : {}", t(&self.language, "error"), err)).size(14).color(Color::from_rgb(0.9, 0.15, 0.15))
            )
            .padding(12)
            .width(Length::Fill)
            .style(error_container_style as fn(&iced::Theme) -> iced::widget::container::Style);

            col = col.push(error_box);
        }

        let btn_submit = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_submit_login")))
            .style(primary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::SubmitLogin)
            .padding(12);

        let btn_back = button(crate::ui::i18n::app_text(&self.language, t(&self.language, "btn_back")))
            .style(secondary_button as fn(&iced::Theme, iced::widget::button::Status) -> iced::widget::button::Style)
            .on_press(Message::BackToWelcome)
            .padding(12);

        col = col.push(row![btn_back, btn_submit].spacing(15)).align_x(Alignment::Center);

        let main_box = Container::new(col).width(Length::Fill).height(Length::Fill).center_x(iced::Length::Fill).center_y(iced::Length::Fill);
        column![top_bar, main_box].width(Length::Fill).height(Length::Fill).into()
    }
}