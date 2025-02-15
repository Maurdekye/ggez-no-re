# ggez-no-re

A collection of custom utilities and modules that extend the base functionality of the Rust ggez game engine.

### Build notes

If building with the `const_logger` feature, make sure to include `#![feature(generic_const_exprs)]` in your crate to prevent a compilation error, until this bug is fixed: https://github.com/rust-lang/rust/issues/133199#issuecomment-2630615573
