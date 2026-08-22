use iced::widget::{button, column, row, text, text_input, Container, Scrollable};
use iced::{Alignment, Color, Element, Length};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn view_active_call(&self) -> Element<'_, Message> {
        let (_, active_pseudo) = self.active_call.as_ref().unwrap();
        let title = text("📞 Appel en cours").size(40);
        let subtitle = text(format!("En communication sécurisée avec {}", active_pseudo)).size(25);

        // --- 1. CONTRÔLES AUDIO ---
        let mute_text = if self.is_muted { "🎙️ Activer le micro" } else { "🔇 Couper le micro (Mute)" };
        let btn_mute = button(mute_text).on_press(Message::ToggleMute).padding(20);
        let btn_hangup = button("❌ Raccrocher").on_press(Message::HangUpCall).padding(20);

        let buttons = row![btn_mute, btn_hangup].spacing(40);

        // --- 2. HISTORIQUE DU CHAT ---
        let mut chat_messages = column![].spacing(10);
        for (author, msg) in &self.chat_history {
            let is_me = author == "Moi";
            let color = if is_me {
                Color::from_rgb(0.2, 0.5, 0.8) // Bleu pour toi
            } else {
                Color::from_rgb(0.8, 0.5, 0.2) // Orange pour l'ami
            };

            let msg_text = text(format!("{}: {}", author, msg)).size(16).style(color);
            chat_messages = chat_messages.push(msg_text);
        }

        // Zone défilante pour l'historique (hauteur fixe pour ne pas écraser l'écran)
        let chat_scroll = Scrollable::new(chat_messages)
            .height(Length::Fixed(250.0))
            .width(Length::Fixed(600.0));

        // --- 3. SAISIE DE TEXTE ---
        let input_chat = text_input("Écrivez un message furtif...", &self.chat_input)
            .on_input(Message::ChatInputChanged)
            .on_submit(Message::SendChatMessage) // Permet d'envoyer avec la touche "Entrée"
            .padding(10);

        let btn_send = button("Envoyer")
            .on_press(Message::SendChatMessage)
            .padding(10);

        let input_row = row![input_chat, btn_send]
            .spacing(10)
            .width(Length::Fixed(600.0));

        let chat_section = column![
            text("💬 Chat Éphémère (Zéro-Trace)").size(20),
            chat_scroll,
            input_row
        ].spacing(15).align_items(Alignment::Center);

        // --- ASSEMBLAGE FINAL ---
        let content = column![title, subtitle, buttons, chat_section]
            .spacing(40)
            .align_items(Alignment::Center);

        Container::new(content).center_x().center_y().into()
    }

    pub(crate) fn view_incoming_call(&self) -> Element<'_, Message> {
        let (caller_id, caller_pseudo, sdp) = self.incoming_call.as_ref().unwrap();
        let title = text("🔔 Appel entrant !").size(40);
        let subtitle = text(format!("{} souhaite communiquer avec vous.", caller_pseudo)).size(25);
        let timer_text = text(format!("(Rejet automatique dans {}s)", 15 - self.incoming_call_timer)).size(16);

        let btn_accept = button("✅ Décrocher").on_press(Message::AcceptCall(caller_id.clone(), sdp.clone())).padding(15);
        let btn_reject = button("❌ Rejeter").on_press(Message::RejectCall(caller_id.clone())).padding(15);

        let buttons = row![btn_reject, btn_accept].spacing(30);
        let content = column![title, subtitle, timer_text, buttons].spacing(25).padding(60).align_items(Alignment::Center);
        Container::new(content).center_x().center_y().into()
    }

    // Le routage principal de la zone connectée
    pub(crate) fn view_unlocked(&self) -> Element<'_, Message> {
        if self.active_call.is_some() {
            return self.view_active_call();
        }
        if self.incoming_call.is_some() {
            return self.view_incoming_call();
        }
        self.view_dashboard()
    }
}