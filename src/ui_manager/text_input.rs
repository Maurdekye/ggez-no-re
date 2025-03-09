use std::{collections::HashSet, time::Instant};

use clipboard_rs::{Clipboard, ClipboardContext};
use ggez::{
    Context, GameResult,
    glam::{Vec2, vec2},
    graphics::{Canvas, Color, DrawMode, Mesh, Text},
    input::mouse::CursorIcon,
    winit::{
        event::MouseButton,
        keyboard::{Key, NamedKey},
    },
};

use crate::util::{AnchorPoint, ContextExt, DrawableWihParamsExt, MinByF32Key, RectExt, TextExt};

use super::{Bounds, UIElementRenderable, UIElementState, TEXTINPUT_BODY, TEXTINPUT_BORDER};

pub const CURSOR_BLINK_INTERVAL: f32 = 1.0;

pub struct TextInput {
    pub bounds: Bounds,
    pub state: UIElementState,
    pub text: String,
    scale: f32,
    focused: bool,
    cursor: usize,
    mask: fn(char) -> bool,
    pub maxlen: Option<usize>,
    last_action: Instant,
}

impl TextInput {
    pub fn new(bounds: Bounds) -> TextInput {
        TextInput::new_masked(bounds, |_| true)
    }

    pub fn new_masked(bounds: Bounds, mask: fn(char) -> bool) -> TextInput {
        TextInput {
            bounds,
            state: UIElementState::Enabled,
            text: String::new(),
            focused: false,
            scale: 16.0,
            cursor: 0,
            mask,
            maxlen: None,
            last_action: Instant::now(),
        }
    }

    fn delete_char(&mut self) {
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
            self.last_action = Instant::now()
        }
    }

    fn backspace_char(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.text.remove(self.cursor);
            self.last_action = Instant::now()
        }
    }

    fn type_char(&mut self, ch: char) {
        if (self.mask)(ch) && self.maxlen.is_none_or(|maxlen| self.text.len() < maxlen) {
            if self.cursor == self.text.len() {
                self.text.push(ch);
            } else {
                self.text.insert(self.cursor, ch);
            }
            self.cursor += 1;
            self.last_action = Instant::now()
        }
    }

    fn left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.last_action = Instant::now()
        }
    }

    fn right(&mut self) {
        if self.cursor < self.text.len() {
            self.cursor += 1;
            self.last_action = Instant::now()
        }
    }

    fn get_drawable_text(&self, ctx: &Context) -> (Text, Vec2) {
        let bounds = self.bounds.corrected_bounds(ctx.res());
        let mut text = Text::new(&self.text);
        text.set_scale(self.scale);
        text.set_wrap(false);
        text.set_bounds(Vec2::from(bounds.size()) - vec2(8.0, 0.0));
        let anchorpoint = bounds.parametric(vec2(0.0, 0.5)) + vec2(4.0, 0.0);
        (text, anchorpoint)
    }

    pub fn draw(&self, ctx: &Context, canvas: &mut Canvas, _mouse: Vec2) -> GameResult<()> {
        if self.state == UIElementState::Invisible {
            return Ok(());
        }

        let bounds = self.bounds.corrected_bounds(ctx.res());
        Mesh::new_rounded_rectangle(ctx, DrawMode::fill(), bounds, 2.0, TEXTINPUT_BODY)?
            .draw(canvas);
        Mesh::new_rounded_rectangle(ctx, DrawMode::stroke(2.0), bounds, 2.0, TEXTINPUT_BORDER)?
            .draw(canvas);
        let (text, text_anchorpoint) = self.get_drawable_text(ctx);
        text.anchored_by(ctx, text_anchorpoint, AnchorPoint::CenterWest)?
            .color(Color::BLACK)
            .draw(canvas);
        if self.focused
            && (Instant::now() - self.last_action).as_secs_f32() % (CURSOR_BLINK_INTERVAL)
                < CURSOR_BLINK_INTERVAL / 2.0
        {
            let origin = text_anchorpoint - vec2(0.0, self.scale / 2.0);
            let cursor_pos: Vec2 = if self.text.is_empty() {
                origin
            } else if self.cursor >= self.text.len() {
                let bounds: Vec2 = text.measure(ctx)?.into();
                origin + vec2(bounds.x, 0.0)
            } else {
                let glyph_positions = text.glyph_positions(ctx)?;
                origin + vec2(glyph_positions[self.cursor].x, 0.0)
            };
            Mesh::new_line(
                ctx,
                &[cursor_pos, cursor_pos + vec2(0.0, self.scale)],
                2.0,
                Color::BLACK,
            )?
            .draw(canvas);
        }
        Ok(())
    }

    pub fn update(
        &mut self,
        ctx: &Context,
        mouse: Vec2,
        just_pressed_keys: &HashSet<Key>,
    ) -> GameResult<Option<CursorIcon>> {
        if self.state != UIElementState::Enabled {
            return Ok(None);
        }
        let mouse_pressed = ctx.mouse.button_just_pressed(MouseButton::Left);
        let mut cursor_override = None;

        let bounds = self.bounds.corrected_bounds(ctx.res());
        if bounds.contains(mouse) {
            cursor_override = Some(CursorIcon::Text);
            if mouse_pressed {
                self.focused = true;
                let (text, anchorpoint) = self.get_drawable_text(ctx);
                let text_bounds: Vec2 = text.measure(ctx)?.into();
                self.cursor = text
                    .glyph_positions(ctx)?
                    .iter()
                    .cloned()
                    .map(Vec2::from)
                    .chain([text_bounds])
                    .enumerate()
                    .min_by_f32_key(|(_, pos)| ((*pos + anchorpoint) - mouse).x.abs())
                    .map_or(0, |(i, _)| i)
            }
        } else if mouse_pressed {
            self.focused = false;
        }

        if self.focused {
            let additional_keys = if ctx.keyboard.is_key_repeated() {
                &ctx.keyboard.pressed_logical_keys
            } else {
                &HashSet::new()
            };
            for key in just_pressed_keys.iter().chain(additional_keys) {
                log::trace!("key = {key:?}");
                match key {
                    Key::Named(NamedKey::Delete) => self.delete_char(),
                    Key::Named(NamedKey::Backspace) => self.backspace_char(),
                    Key::Named(NamedKey::ArrowRight) => self.right(),
                    Key::Named(NamedKey::ArrowLeft) => self.left(),
                    Key::Character(ch) => {
                        if (ch == "v"
                            || (!ctx
                                .keyboard
                                .is_logical_key_pressed(&Key::Named(NamedKey::Shift))
                                && ch == "V"))
                            && ctx
                                .keyboard
                                .is_logical_key_pressed(&Key::Named(NamedKey::Control))
                        {
                            let clipboard_contents = ClipboardContext::new()
                                .unwrap()
                                .get_text()
                                .unwrap_or_default();
                            for chr in clipboard_contents.chars() {
                                self.type_char(chr);
                            }
                        } else {
                            for c in ch.chars() {
                                self.type_char(c);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(cursor_override)
    }
}

impl UIElementRenderable for TextInput {
    fn get_corrected_bounds(&self, ctx: &Context) -> ggez::graphics::Rect {
        self.bounds.corrected_bounds(ctx.res())
    }

    fn get_state(&self) -> UIElementState {
        self.state
    }
}