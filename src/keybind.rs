use std::fmt::Display;

use ggez::{
    Context,
    winit::{
        event::MouseButton,
        keyboard::{Key, NamedKey},
    },
};
use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! keybinds {
    (pub struct $structname:ident {$($name:ident: $key:expr,)*}) => {
        #[derive(Clone, Debug, ::serde::Serialize, ::serde::Deserialize)]
        pub struct $structname {
            $(pub $name: $crate::keybind::Keybind,)*
        }

        impl Default for $structname {
            fn default() -> $structname {
                $structname {
                    $($name: $key.into(),)*
                }
            }
        }
    };
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Keybind {
    Mouse(MouseButton),
    Key(Key),
}

impl Keybind {
    pub fn pressed(&self, ctx: &Context) -> bool {
        match self {
            Keybind::Mouse(mouse_button) => ctx.mouse.button_pressed(*mouse_button),
            Keybind::Key(key) => ctx.keyboard.is_logical_key_pressed(key),
        }
    }

    pub fn just_pressed(&self, ctx: &Context) -> bool {
        match self {
            Keybind::Mouse(mouse_button) => ctx.mouse.button_just_pressed(*mouse_button),
            Keybind::Key(key) => ctx.keyboard.is_logical_key_just_pressed(key),
        }
    }

    #[allow(unused)]
    pub fn just_released(&self, ctx: &Context) -> bool {
        match self {
            Keybind::Mouse(mouse_button) => ctx.mouse.button_just_released(*mouse_button),
            Keybind::Key(key) => ctx.keyboard.is_logical_key_just_released(key),
        }
    }
}

impl From<MouseButton> for Keybind {
    fn from(value: MouseButton) -> Self {
        Keybind::Mouse(value)
    }
}

impl From<&str> for Keybind {
    fn from(value: &str) -> Self {
        Keybind::Key(Key::Character(value.into()))
    }
}

impl From<NamedKey> for Keybind {
    fn from(value: NamedKey) -> Self {
        Keybind::Key(Key::Named(value))
    }
}

impl Display for Keybind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Keybind::Mouse(mouse_button) => write!(f, "{:?} Mouse", mouse_button),
            Keybind::Key(key) => match key {
                Key::Named(named_key) => match named_key {
                    NamedKey::ArrowUp => write!(f, "↑"),
                    NamedKey::ArrowDown => write!(f, "↓"),
                    NamedKey::ArrowLeft => write!(f, "←"),
                    NamedKey::ArrowRight => write!(f, "→"),
                    named_key => write!(f, "{named_key:?}"),
                },
                Key::Character(ch) => write!(f, "{}", ch.to_uppercase()),
                key => match key.to_text() {
                    Some(text) => write!(f, "{}", text.to_uppercase()),
                    None => write!(f, "{key:?}"),
                },
            },
        }
    }
}
