use std::collections::HashMap;

use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub accent: Color,
    pub selection: Color,
    pub text: Color,
    pub text_dim: Color,
    pub background: Color,
    pub surface: Color,
    pub border: Color,
    pub success: Color,
    pub danger: Color,
    pub warning: Color,
}

impl Theme {
    pub const fn new() -> Self {
        Self {
            accent: Color::Cyan,
            selection: Color::Indexed(33),
            text: Color::White,
            text_dim: Color::Gray,
            background: Color::Reset,
            surface: Color::Reset,
            border: Color::DarkGray,
            success: Color::Green,
            danger: Color::Red,
            warning: Color::Yellow,
        }
    }

    pub fn terminal() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    Terminal,
    Monochrome,
    Amoled,
    CatppuccinMocha,
    GruvboxDark,
    Dracula,
    Nord,
    SolarizedDark,
    TokyoNight,
    OneDark,
    RosePine,
}

impl ThemePreset {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().replace('-', "").replace('_', "").as_str() {
            "terminal" => Some(Self::Terminal),
            "monochrome" => Some(Self::Monochrome),
            "amoled" => Some(Self::Amoled),
            "catppuccin" | "catppuccinmocha" => Some(Self::CatppuccinMocha),
            "gruvbox" | "gruvboxdark" => Some(Self::GruvboxDark),
            "dracula" => Some(Self::Dracula),
            "nord" => Some(Self::Nord),
            "solarized" | "solarizeddark" => Some(Self::SolarizedDark),
            "tokyo" | "tokyonight" => Some(Self::TokyoNight),
            "onedark" | "one-dark" => Some(Self::OneDark),
            "rosepine" | "rose-pine" => Some(Self::RosePine),
            _ => None,
        }
    }

    pub fn colors(&self) -> Theme {
        match self {
            Self::Terminal => Theme::terminal(),
            Self::Monochrome => Theme {
                accent: Color::White,
                selection: Color::DarkGray,
                text: Color::White,
                text_dim: Color::Gray,
                background: Color::Indexed(234),
                surface: Color::Indexed(236),
                border: Color::Gray,
                success: Color::Gray,
                danger: Color::DarkGray,
                warning: Color::White,
            },
            Self::Amoled => Theme {
                accent: Color::Green,
                selection: Color::Indexed(22),
                text: Color::Green,
                text_dim: Color::Indexed(28),
                background: Color::Indexed(16),
                surface: Color::Indexed(232),
                border: Color::Indexed(22),
                success: Color::Green,
                danger: Color::Red,
                warning: Color::Yellow,
            },
            Self::CatppuccinMocha => Theme {
                accent: Color::Indexed(183),
                selection: Color::Indexed(60),
                text: Color::Indexed(252),
                text_dim: Color::Indexed(244),
                background: Color::Indexed(235),
                surface: Color::Indexed(237),
                border: Color::Indexed(59),
                success: Color::Indexed(114),
                danger: Color::Indexed(167),
                warning: Color::Indexed(214),
            },
            Self::GruvboxDark => Theme {
                accent: Color::Indexed(214),
                selection: Color::Indexed(237),
                text: Color::Indexed(223),
                text_dim: Color::Indexed(246),
                background: Color::Indexed(235),
                surface: Color::Indexed(237),
                border: Color::Indexed(239),
                success: Color::Indexed(142),
                danger: Color::Indexed(124),
                warning: Color::Indexed(214),
            },
            Self::Dracula => Theme {
                accent: Color::Indexed(141),
                selection: Color::Indexed(59),
                text: Color::Indexed(188),
                text_dim: Color::Indexed(102),
                background: Color::Indexed(235),
                surface: Color::Indexed(237),
                border: Color::Indexed(59),
                success: Color::Indexed(84),
                danger: Color::Indexed(203),
                warning: Color::Indexed(221),
            },
            Self::Nord => Theme {
                accent: Color::Indexed(110),
                selection: Color::Indexed(60),
                text: Color::Indexed(188),
                text_dim: Color::Indexed(145),
                background: Color::Indexed(236),
                surface: Color::Indexed(59),
                border: Color::Indexed(60),
                success: Color::Indexed(114),
                danger: Color::Indexed(167),
                warning: Color::Indexed(214),
            },
            Self::SolarizedDark => Theme {
                accent: Color::Indexed(33),
                selection: Color::Indexed(60),
                text: Color::Indexed(145),
                text_dim: Color::Indexed(102),
                background: Color::Indexed(234),
                surface: Color::Indexed(235),
                border: Color::Indexed(240),
                success: Color::Indexed(64),
                danger: Color::Indexed(124),
                warning: Color::Indexed(136),
            },
            Self::TokyoNight => Theme {
                accent: Color::Indexed(111),
                selection: Color::Indexed(60),
                text: Color::Indexed(146),
                text_dim: Color::Indexed(103),
                background: Color::Indexed(234),
                surface: Color::Indexed(236),
                border: Color::Indexed(59),
                success: Color::Indexed(114),
                danger: Color::Indexed(203),
                warning: Color::Indexed(214),
            },
            Self::OneDark => Theme {
                accent: Color::Indexed(75),
                selection: Color::Indexed(59),
                text: Color::Indexed(249),
                text_dim: Color::Indexed(243),
                background: Color::Indexed(235),
                surface: Color::Indexed(237),
                border: Color::Indexed(239),
                success: Color::Indexed(114),
                danger: Color::Indexed(167),
                warning: Color::Indexed(215),
            },
            Self::RosePine => Theme {
                accent: Color::Indexed(183),
                selection: Color::Indexed(60),
                text: Color::Indexed(188),
                text_dim: Color::Indexed(103),
                background: Color::Indexed(234),
                surface: Color::Indexed(236),
                border: Color::Indexed(59),
                success: Color::Indexed(150),
                danger: Color::Indexed(167),
                warning: Color::Indexed(215),
            },
        }
    }
}

pub fn parse_theme(config_theme: &Option<String>, overrides: &HashMap<String, String>) -> Theme {
    let preset = config_theme
        .as_deref()
        .and_then(ThemePreset::from_str)
        .unwrap_or(ThemePreset::Terminal);
    let mut theme = preset.colors();

    for (key, value) in overrides {
        if let Some(color) = parse_color(value) {
            match key.to_lowercase().as_str() {
                "accent" => theme.accent = color,
                "selection" => theme.selection = color,
                "text" => theme.text = color,
                "text_dim" => theme.text_dim = color,
                "background" => theme.background = color,
                "surface" => theme.surface = color,
                "border" => theme.border = color,
                "success" => theme.success = color,
                "danger" => theme.danger = color,
                "warning" => theme.warning = color,
                _ => {}
            }
        }
    }

    theme
}

fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }
    match s.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "reset" => Some(Color::Reset),
        _ => None,
    }
}
