#![allow(incomplete_features)]
#![doc = include_str!("../README.md")]
#![feature(path_add_extension)]
#![feature(try_blocks)]
#![feature(generic_const_exprs)]
pub mod checker_spiral;
pub mod csv_recorder;
pub mod discord;
pub mod keybind;
pub mod line;
pub mod logger;
pub mod persist;
pub mod shader_scene;
pub mod sub_event_handler;
pub mod transport;
pub mod ui_manager;
pub mod util;
