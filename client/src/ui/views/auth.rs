use iced::widget::{button, column, container, row, text, text_input, Container};
use iced::{Alignment, Background, Border, Color, Element, Shadow, Theme};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

struct ErrorContainerStyle;

impl container::StyleSheet for ErrorContainerStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(Color::from_rgb(0.9, 0.15, 0.15)),
            background: Some(Background::Color(Color::from_rgb(1.0, 0.9, 0.9))),
            border: Border { color: Color::from_rgb(0.9, 0.15, 0.15), width: 1.5, radius: 5.0.into() },
            shadow: Shadow::default(),
        }
    }
}

impl KakolookiyamApp {
    pub(crate) fn clear_auth_fields(&mut self) {
        self.pseudo_input.clear();
        self.password_input.clear();
        self.password_confirm_input.clear();
        self.auth_error = None;
    }

    pub(crate) fn view_welcome(&self) -> Element<'_, Message> {
        let title = text("Kakolookiyam").size(45);
        let subtitle = text("Communication P2P Zéro-Trace").size(18);
        let btn_create = button("Créer un nouveau compte (Coffre-fort local)").on_press(Message::GoToCreateAccount).padding(15);
        let btn_login = button("Se connecter (Déverrouiller un compte existant)").on_press(Message::GoToLogin).padding(15);
        let content = column![title, subtitle, btn_create, btn_login].spacing(25).padding(60).align_items(Alignment::Center);
        Container::new(content).center_x().center_y().into()
    }

    pub(crate) fn view_create_account(&self) -> Element<'_, Message> {
        let title = text("🔐 Créer un compte").size(35);
        let info = text("Votre mot de passe chiffrera votre clé privée et votre pseudo.");
        let pseudo_input = text_input("Choisissez un pseudo...", &self.pseudo_input).on_input(Message::PseudoChanged).padding(15);
        let pass_input = text_input("Nouveau mot de passe (min. 12 car., Maj, Min, Chiffre, Spécial)...", &self.password_input).on_input(Message::PasswordChanged).secure(true).padding(15);
        let pass_confirm = text_input("Confirmez le mot de passe...", &self.password_confirm_input).on_input(Message::PasswordConfirmChanged).secure(true).padding(15);

        let mut col = column![title, info, pseudo_input, pass_input, pass_confirm].spacing(20);

        if let Some(err) = &self.auth_error {
            let error_box = container(text(format!("❌ Erreur : {}", err)).size(14).style(Color::from_rgb(0.9, 0.15, 0.15)))
                .padding(12).width(iced::Length::Fill).style(iced::theme::Container::Custom(Box::new(ErrorContainerStyle)));
            col = col.push(error_box);
        }

        let btn_submit = button("Créer et Chiffrer").on_press(Message::SubmitCreateAccount).padding(12);
        let btn_back = button("Retour").on_press(Message::BackToWelcome).padding(12);
        col = col.push(row![btn_back, btn_submit].spacing(15)).align_items(Alignment::Center);

        Container::new(col).center_x().center_y().into()
    }

    pub(crate) fn view_login(&self) -> Element<'_, Message> {
        let title = text("🔓 Déverrouiller le coffre-fort").size(35);
        let pseudo_input = text_input("Votre pseudo...", &self.pseudo_input).on_input(Message::PseudoChanged).padding(15);
        let pass_input = text_input("Votre mot de passe maître...", &self.password_input).on_input(Message::PasswordChanged).secure(true).padding(15);

        let mut col = column![title, pseudo_input, pass_input].spacing(20);

        if let Some(err) = &self.auth_error {
            let error_box = container(text(format!("❌ Erreur : {}", err)).size(14).style(Color::from_rgb(0.9, 0.15, 0.15)))
                .padding(12).width(iced::Length::Fill).style(iced::theme::Container::Custom(Box::new(ErrorContainerStyle)));
            col = col.push(error_box);
        }

        let btn_submit = button("Déverrouiller").on_press(Message::SubmitLogin).padding(12);
        let btn_back = button("Retour").on_press(Message::BackToWelcome).padding(12);
        col = col.push(row![btn_back, btn_submit].spacing(15)).align_items(Alignment::Center);

        Container::new(col).center_x().center_y().into()
    }
}