use iced::widget::{button, container};
use iced::{Color, Theme, Background, Vector, Border, Shadow};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// --- COULEURS FIXES ---
pub const ACCENT_RED_DARK: Color = Color::from_rgb(0.35, 0.05, 0.05);
pub const MOI_RED: Color = Color::from_rgb(0.85, 0.15, 0.15);
pub const ACCENT_TEAL: Color = Color::from_rgb(0.10, 0.50, 0.45);

// --- COULEUR DYNAMIQUE ---
pub fn dynamic_accent(theme: &Theme) -> Color {
    match theme {
        Theme::Light => ACCENT_TEAL,
        _ => MOI_RED,
    }
}

// --- TEXTES DYNAMIQUES ---
pub fn text_main(theme: &Theme) -> Color {
    match theme {
        Theme::Light => Color::BLACK,
        _ => Color::WHITE,
    }
}

pub fn text_muted(theme: &Theme) -> Color {
    match theme {
        Theme::Light => Color::from_rgb(0.25, 0.25, 0.25),
        _ => Color::from_rgb(0.70, 0.70, 0.75),
    }
}

pub fn color_for_user(pseudo: &str, theme: &Theme) -> Color {
    if pseudo == "Moi" { return MOI_RED; }

    let mut hasher = DefaultHasher::new();
    pseudo.hash(&mut hasher);
    let hash = hasher.finish();
    let step = (hash % 10) as f32;

    match theme {
        Theme::Light => {
            let hue = 260.0 + (step * 8.0);
            color_from_hsl(hue, 0.85, 0.35)
        },
        _ => {
            let hue = 170.0 + (step * 8.0);
            color_from_hsl(hue, 0.85, 0.65)
        }
    }
}

fn color_from_hsl(h: f32, s: f32, l: f32) -> Color {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0), 60..=119 => (x, c, 0.0), 120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c), 240..=299 => (x, 0.0, c), _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}

pub struct PrimaryButton;
impl button::StyleSheet for PrimaryButton {
    type Style = Theme;
    fn active(&self, theme: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(dynamic_accent(theme))),
            text_color: Color::WHITE,
            border: Border { radius: 4.0.into(), width: 0.0, color: Color::TRANSPARENT },
            shadow_offset: Vector::new(0.0, 0.0), shadow: Shadow::default(),
        }
    }
    fn hovered(&self, theme: &Self::Style) -> button::Appearance { self.active(theme) }
}

pub struct HangupButton;
impl button::StyleSheet for HangupButton {
    type Style = Theme;
    fn active(&self, _theme: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(ACCENT_RED_DARK)),
            text_color: Color::WHITE,
            border: Border { radius: 4.0.into(), width: 0.0, color: Color::TRANSPARENT },
            shadow_offset: Vector::new(0.0, 0.0), shadow: Shadow::default(),
        }
    }
    fn hovered(&self, theme: &Self::Style) -> button::Appearance { self.active(theme) }
}

pub struct SecondaryButton;
impl button::StyleSheet for SecondaryButton {
    type Style = Theme;
    fn active(&self, theme: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: match theme {
                Theme::Light => Some(Background::Color(Color::from_rgb(0.85, 0.85, 0.85))),
                _ => Some(Background::Color(Color::from_rgb(0.2, 0.2, 0.2))),
            },
            text_color: text_main(theme),
            border: Border { radius: 4.0.into(), width: 0.0, color: Color::TRANSPARENT },
            shadow_offset: Vector::new(0.0, 0.0), shadow: Shadow::default(),
        }
    }
    fn hovered(&self, theme: &Self::Style) -> button::Appearance { self.active(theme) }
}

pub struct SidebarButton { pub is_selected: bool }
impl button::StyleSheet for SidebarButton {
    type Style = Theme;
    fn active(&self, theme: &Self::Style) -> button::Appearance {
        let accent = dynamic_accent(theme);
        button::Appearance {
            background: if self.is_selected { Some(Background::Color(Color::from_rgba(accent.r, accent.g, accent.b, 0.15))) } else { None },
            text_color: if self.is_selected { accent } else { text_main(theme) },
            border: Border { radius: 4.0.into(), width: 0.0, color: Color::TRANSPARENT },
            shadow_offset: Vector::new(0.0, 0.0), shadow: Shadow::default(),
        }
    }
    fn hovered(&self, theme: &Self::Style) -> button::Appearance {
        let mut app = self.active(theme);
        if !self.is_selected {
            app.background = match theme {
                Theme::Light => Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.05))),
                _ => Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.05))),
            };
        }
        app
    }
}

pub struct ErrorContainerStyle;
impl container::StyleSheet for ErrorContainerStyle {
    type Style = Theme;
    fn appearance(&self, _theme: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(MOI_RED),
            background: Some(Background::Color(Color::from_rgba(0.85, 0.15, 0.15, 0.1))),
            border: Border { color: MOI_RED, width: 1.5, radius: 5.0.into() },
            shadow: Shadow::default(),
        }
    }
}

// --- NOUVEAU : Style de la fenêtre Pop-Up (Overlay) ---
pub struct OverlayContainerStyle;
impl container::StyleSheet for OverlayContainerStyle {
    type Style = Theme;
    fn appearance(&self, theme: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(text_main(theme)),
            background: match theme {
                Theme::Light => Some(Background::Color(Color::from_rgb(0.95, 0.95, 0.95))),
                _ => Some(Background::Color(Color::from_rgb(0.12, 0.12, 0.12))),
            },
            border: Border { color: dynamic_accent(theme), width: 1.0, radius: 8.0.into() },
            shadow: Shadow::default(),
        }
    }
}