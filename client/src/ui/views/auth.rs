use iced::widget::{button, column, container, row, text, text_input, Container};
use iced::{alignment, Alignment, Color, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;
use crate::ui::theme::{text_main, text_muted, PrimaryButton, SecondaryButton, ErrorContainerStyle};
use crate::ui::i18n::t;

impl KakolookiyamApp {
    pub(crate) fn clear_auth_fields(&mut self) {
        self.pseudo_input.clear();
        self.password_input.clear();
        self.password_confirm_input.clear();
        self.auth_error = None;
    }

    pub(crate) fn view_welcome(&self) -> Element<'_, Message> {
        let lang_btn = button(text(t(&self.language, "lang_toggle")))
            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
            .on_press(Message::ToggleLanguage)
            .padding(10);
        let top_bar = container(lang_btn).width(Length::Fill).align_x(alignment::Horizontal::Right).padding(20);

        let title = text("Kakolookiyam")
            .size(45)
            .style(text_main(&self.current_theme))
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Center);

        let subtitle = text(t(&self.language, "welcome_sub"))
            .size(18)
            .style(text_muted(&self.current_theme))
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Center);

        let btn_create = button(text(t(&self.language, "btn_create")))
            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
            .on_press(Message::GoToCreateAccount)
            .padding(15);

        let btn_login = button(text(t(&self.language, "btn_login")))
            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
            .on_press(Message::GoToLogin)
            .padding(15);

        let content = column![title, subtitle, btn_create, btn_login]
            .spacing(25)
            .padding(60)
            .align_items(Alignment::Center)
            .width(Length::Fill);

        let main_box = Container::new(content).width(Length::Fill).height(Length::Fill).center_x().center_y();
        column![top_bar, main_box].width(Length::Fill).height(Length::Fill).into()
    }

    pub(crate) fn view_create_account(&self) -> Element<'_, Message> {
        let lang_btn = button(text(t(&self.language, "lang_toggle")))
            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
            .on_press(Message::ToggleLanguage)
            .padding(10);
        let top_bar = container(lang_btn).width(Length::Fill).align_x(alignment::Horizontal::Right).padding(20);

        let title = column![
            text("KAKOLOOKIYAM")
                .size(45)
                .style(text_main(&self.current_theme))
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center),
            text(t(&self.language, "create_vault_title"))
                .size(30)
                .style(text_main(&self.current_theme))
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
        ].spacing(10);

        let info = text(t(&self.language, "create_vault_info"))
            .style(text_muted(&self.current_theme))
            .width(Length::Fill)
            .horizontal_alignment(alignment::Horizontal::Center);

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
                text(format!("{} : {}", t(&self.language, "error"), err)).size(14).style(Color::from_rgb(0.9, 0.15, 0.15))
            )
            .padding(12)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(ErrorContainerStyle)));

            col = col.push(error_box);
        }

        let btn_submit = button(text(t(&self.language, "btn_submit_create")))
            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
            .on_press(Message::SubmitCreateAccount)
            .padding(12);

        let btn_back = button(text(t(&self.language, "btn_back")))
            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
            .on_press(Message::BackToWelcome)
            .padding(12);

        col = col.push(row![btn_back, btn_submit].spacing(15)).align_items(Alignment::Center);

        let main_box = Container::new(col).width(Length::Fill).height(Length::Fill).center_x().center_y();
        column![top_bar, main_box].width(Length::Fill).height(Length::Fill).into()
    }

    pub(crate) fn view_login(&self) -> Element<'_, Message> {
        let lang_btn = button(text(t(&self.language, "lang_toggle")))
            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
            .on_press(Message::ToggleLanguage)
            .padding(10);
        let top_bar = container(lang_btn).width(Length::Fill).align_x(alignment::Horizontal::Right).padding(20);

        let title = column![
            text("KAKOLOOKIYAM")
                .size(45)
                .style(text_main(&self.current_theme))
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center),
            text(t(&self.language, "login_title"))
                .size(30)
                .style(text_main(&self.current_theme))
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
        ].spacing(10);

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
                text(format!("{} : {}", t(&self.language, "error"), err)).size(14).style(Color::from_rgb(0.9, 0.15, 0.15))
            )
            .padding(12)
            .width(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(ErrorContainerStyle)));

            col = col.push(error_box);
        }

        let btn_submit = button(text(t(&self.language, "btn_submit_login")))
            .style(iced::theme::Button::Custom(Box::new(PrimaryButton)))
            .on_press(Message::SubmitLogin)
            .padding(12);

        let btn_back = button(text(t(&self.language, "btn_back")))
            .style(iced::theme::Button::Custom(Box::new(SecondaryButton)))
            .on_press(Message::BackToWelcome)
            .padding(12);

        col = col.push(row![btn_back, btn_submit].spacing(15)).align_items(Alignment::Center);

        let main_box = Container::new(col).width(Length::Fill).height(Length::Fill).center_x().center_y();
        column![top_bar, main_box].width(Length::Fill).height(Length::Fill).into()
    }
}