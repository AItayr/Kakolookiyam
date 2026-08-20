use iced::widget::{button, column, row, text, text_input, Container};
use iced::{Alignment, Element};

use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

// Nous ajoutons ces fonctions graphiques à notre structure KakolookiyamApp
impl KakolookiyamApp {

    pub(crate) fn clear_auth_fields(&mut self) {
        self.password_input.clear();
        self.password_confirm_input.clear();
        self.auth_error = None;
    }

    pub(crate) fn view_welcome(&self) -> Element<'_, Message> {
        let title = text("Kakolookiyam").size(45);
        let subtitle = text("Communication P2P Zéro-Trace").size(18);

        let btn_create = button("Créer un nouveau compte (Coffre-fort local)")
            .on_press(Message::GoToCreateAccount)
            .padding(15);

        let btn_login = button("Se connecter (Déverrouiller un compte existant)")
            .on_press(Message::GoToLogin)
            .padding(15);

        let content = column![title, subtitle, btn_create, btn_login]
            .spacing(25)
            .padding(60)
            .align_items(Alignment::Center);

        Container::new(content).center_x().center_y().into()
    }

    pub(crate) fn view_create_account(&self) -> Element<'_, Message> {
        let title = text("🔐 Créer un compte").size(35);
        let info = text("Ce mot de passe chiffrera votre clé locale. Ne le perdez pas !");

        let pass_input = text_input("Nouveau mot de passe...", &self.password_input)
            .on_input(Message::PasswordChanged)
            .secure(true).padding(15);

        let pass_confirm = text_input("Confirmez le mot de passe...", &self.password_confirm_input)
            .on_input(Message::PasswordConfirmChanged)
            .secure(true).padding(15);

        let mut col = column![title, info, pass_input, pass_confirm].spacing(20);

        if let Some(err) = &self.auth_error {
            col = col.push(text(err));
        }

        let btn_submit = button("Créer et Chiffrer").on_press(Message::SubmitCreateAccount).padding(12);
        let btn_back = button("Retour").on_press(Message::BackToWelcome).padding(12);

        let buttons = row![btn_back, btn_submit].spacing(15);
        col = col.push(buttons).align_items(Alignment::Center);

        Container::new(col).center_x().center_y().into()
    }

    pub(crate) fn view_login(&self) -> Element<'_, Message> {
        let title = text("🔓 Déverrouiller le coffre-fort").size(35);

        let pass_input = text_input("Votre mot de passe maître...", &self.password_input)
            .on_input(Message::PasswordChanged)
            .secure(true).padding(15);

        let mut col = column![title, pass_input].spacing(20);

        if let Some(err) = &self.auth_error {
            col = col.push(text(err));
        }

        let btn_submit = button("Déverrouiller").on_press(Message::SubmitLogin).padding(12);
        let btn_back = button("Retour").on_press(Message::BackToWelcome).padding(12);

        let buttons = row![btn_back, btn_submit].spacing(15);
        col = col.push(buttons).align_items(Alignment::Center);

        Container::new(col).center_x().center_y().into()
    }

    pub(crate) fn view_unlocked(&self) -> Element<'_, Message> {
        let title = text("🛡️ Kakolookiyam P2P").size(28);

        let my_id_display = text(format!("🔑 Mon ID : {}", self.my_local_id)).size(16);
        let copy_button = button("Copier").on_press(Message::CopyIdClicked);
        let identity_row = row![my_id_display, copy_button].spacing(10);

        let status = text(&self.status_message);

        let input = text_input("Entrer l'ID (Clé Publique) de l'ami...", &self.peer_id_input)
            .on_input(Message::PeerIdChanged)
            .padding(10);

        let connect_button = button("Lancer l'appel sécurisé")
            .on_press(Message::ConnectClicked);

        let content = column![title, identity_row, status, input, connect_button]
            .spacing(20)
            .padding(40)
            .align_items(Alignment::Center);

        Container::new(content).center_x().center_y().into()
    }
}