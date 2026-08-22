use iced::widget::{button, column, row, text, Container};
use iced::{Alignment, Element};
use crate::ui::app::KakolookiyamApp;
use crate::ui::messages::Message;

impl KakolookiyamApp {
    pub(crate) fn view_active_call(&self) -> Element<'_, Message> {
        let (_, active_pseudo) = self.active_call.as_ref().unwrap();
        let title = text("📞 Appel en cours").size(40);
        let subtitle = text(format!("En communication sécurisée avec {}", active_pseudo)).size(25);

        let mute_text = if self.is_muted { "🎙️ Activer le micro" } else { "🔇 Couper le micro (Mute)" };
        let btn_mute = button(mute_text).on_press(Message::ToggleMute).padding(20);
        let btn_hangup = button("❌ Raccrocher").on_press(Message::HangUpCall).padding(20);

        let buttons = row![btn_mute, btn_hangup].spacing(40);
        let content = column![title, subtitle, buttons].spacing(40).align_items(Alignment::Center);
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