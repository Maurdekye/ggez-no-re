use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::mpsc::Sender, time::Instant};

use clipboard_rs::{Clipboard, ClipboardContext};
use ggez::{
    Context, GameError, GameResult,
    glam::{Vec2, vec2},
    graphics::{Canvas, Color, DrawMode, DrawParam, Mesh, Rect, Text},
    input::mouse::{CursorIcon, set_cursor_type},
    winit::{
        event::MouseButton,
        keyboard::{Key, NamedKey},
    },
};

use crate::{
    sub_event_handler::SubEventHandler,
    util::{
        AnchorPoint, ContextExt, DrawableWihParamsExt, MinByF32Key, RectExt, TextExt, color_mul,
    },
};

pub const TEXTINPUT_BODY: Color = Color {
    r: 0.94,
    g: 0.89,
    b: 0.91,
    a: 1.0,
};

pub const TEXTINPUT_BORDER: Color = Color {
    r: 0.4,
    g: 0.4,
    b: 0.4,
    a: 1.0,
};

pub const BUTTON_COLOR: Color = Color {
    r: 0.5,
    g: 0.5,
    b: 0.5,
    a: 1.0,
};

pub const CURSOR_BLINK_INTERVAL: f32 = 1.0;

#[derive(Debug)]
pub struct Bounds {
    pub relative: Rect,
    pub absolute: Rect,
}

impl Bounds {
    #[allow(unused)]
    pub fn relative(bounds: Rect) -> Bounds {
        Bounds {
            relative: bounds,
            absolute: Rect::new(0.0, 0.0, 0.0, 0.0),
        }
    }

    #[allow(unused)]
    pub fn absolute(bounds: Rect) -> Bounds {
        Bounds {
            relative: Rect::new(0.0, 0.0, 0.0, 0.0),
            absolute: bounds,
        }
    }

    pub fn corrected_bounds(&self, res: Vec2) -> Rect {
        let Bounds {
            relative: relative_bounds,
            absolute: absolute_bounds,
        } = self;
        Rect::new(
            relative_bounds.x * res.x + absolute_bounds.x,
            relative_bounds.y * res.y + absolute_bounds.y,
            relative_bounds.w * res.x + absolute_bounds.w,
            relative_bounds.h * res.y + absolute_bounds.h,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIElementState {
    Enabled,
    Disabled,
    Invisible,
}

impl UIElementState {
    pub fn disabled_if(is_disabled: bool) -> UIElementState {
        if is_disabled {
            UIElementState::Disabled
        } else {
            UIElementState::Enabled
        }
    }

    pub fn invisible_if(is_invisible: bool) -> UIElementState {
        if is_invisible {
            UIElementState::Invisible
        } else {
            UIElementState::Enabled
        }
    }
}

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
}

#[derive(Debug)]
pub struct Button<E> {
    pub bounds: Bounds,
    pub text: Text,
    pub color: Color,
    text_drawparam: DrawParam,
    pub event: E,
    pub state: UIElementState,
}

impl<E> Button<E> {
    pub fn new_with_styling(
        bounds: Bounds,
        text: Text,
        text_drawparam: DrawParam,
        color: Color,
        event: E,
    ) -> Button<E> {
        Button {
            bounds,
            text,
            text_drawparam,
            color,
            event,
            state: UIElementState::Enabled,
        }
    }

    pub fn new(bounds: Bounds, text: Text, event: E) -> Button<E> {
        Button::new_with_styling(bounds, text, DrawParam::default(), BUTTON_COLOR, event)
    }

    pub fn corrected_bounds(&self, res: Vec2) -> Rect {
        self.bounds.corrected_bounds(res)
    }
}

#[derive(Clone)]
pub enum UIElement<B, T> {
    Button(B),
    TextInput(T),
}

impl<B, T> UIElement<B, T> {
    pub fn unwrap_button(self) -> B {
        let UIElement::Button(button) = self else {
            panic!()
        };
        button
    }

    #[allow(unused)]
    pub fn unwrap_text_input(self) -> T {
        let UIElement::TextInput(text_input) = self else {
            panic!()
        };
        text_input
    }
}

pub struct UIManager<E, T = E> {
    #[allow(clippy::type_complexity)]
    elements: Vec<UIElement<Rc<RefCell<Button<E>>>, Rc<RefCell<TextInput>>>>,
    pub cursor_override: Option<CursorIcon>,
    event_sender: Sender<T>,
    mouse_position: Vec2,
    last_pressed_keys: HashSet<Key>,
}

impl<E, T> UIManager<E, T>
where
    T: From<E>,
{
    #[allow(clippy::type_complexity)]
    pub fn new_and_rc_elements<const N: usize>(
        event_sender: Sender<T>,
        elements: [UIElement<Button<E>, TextInput>; N],
    ) -> (
        UIManager<E, T>,
        [UIElement<Rc<RefCell<Button<E>>>, Rc<RefCell<TextInput>>>; N],
    ) {
        let return_elements = elements.map(|elem| match elem {
            UIElement::Button(button) => UIElement::Button(Rc::new(RefCell::new(button))),
            UIElement::TextInput(text_input) => {
                UIElement::TextInput(Rc::new(RefCell::new(text_input)))
            }
        });

        let elements = return_elements.clone().into();
        (
            UIManager {
                elements,
                cursor_override: None,
                event_sender,
                mouse_position: Vec2::ZERO,
                last_pressed_keys: HashSet::new(),
            },
            return_elements,
        )
    }

    pub fn new<const N: usize>(
        event_sender: Sender<T>,
        elements: [UIElement<Button<E>, TextInput>; N],
    ) -> UIManager<E, T> {
        Self::new_and_rc_elements(event_sender, elements).0
    }
}

impl<E, T> SubEventHandler for UIManager<E, T>
where
    E: Clone,
    T: From<E>,
{
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> Result<(), GameError> {
        let res: Vec2 = ctx.gfx.drawable_size().into();
        for element in self.elements.iter() {
            match element {
                UIElement::Button(button) => {
                    let button = button.borrow();
                    if button.state == UIElementState::Invisible {
                        continue;
                    }

                    let bounds = button.bounds.corrected_bounds(res);
                    let contains = bounds.contains(self.mouse_position);
                    let color = match (
                        &button.state,
                        contains,
                        ctx.mouse.button_pressed(MouseButton::Left),
                    ) {
                        (UIElementState::Disabled, _, _) => <[f32; 4]>::from(button.color)
                            .map(|x| (x - 0.5) * 0.25 + 0.5)
                            .into(),
                        (_, true, true) => color_mul(button.color, 0.8),
                        (_, true, _) => color_mul(button.color, 1.2),
                        _ => button.color,
                    };
                    Mesh::new_rounded_rectangle(ctx, DrawMode::fill(), bounds, 5.0, color)?
                        .draw(canvas);
                    button
                        .text
                        .with_params(button.text_drawparam)
                        .centered_on(ctx, bounds.center().into())?
                        .draw(canvas);
                }
                UIElement::TextInput(text_input) => {
                    let text_input = text_input.borrow();
                    if text_input.state == UIElementState::Invisible {
                        continue;
                    }

                    let bounds = text_input.bounds.corrected_bounds(res);
                    Mesh::new_rounded_rectangle(
                        ctx,
                        DrawMode::fill(),
                        bounds,
                        2.0,
                        TEXTINPUT_BODY,
                    )?
                    .draw(canvas);
                    Mesh::new_rounded_rectangle(
                        ctx,
                        DrawMode::stroke(2.0),
                        bounds,
                        2.0,
                        TEXTINPUT_BORDER,
                    )?
                    .draw(canvas);
                    let (text, text_anchorpoint) = text_input.get_drawable_text(ctx);
                    text.anchored_by(ctx, text_anchorpoint, AnchorPoint::CenterWest)?
                        .color(Color::BLACK)
                        .draw(canvas);
                    if text_input.focused
                        && (Instant::now() - text_input.last_action).as_secs_f32()
                            % (CURSOR_BLINK_INTERVAL)
                            < CURSOR_BLINK_INTERVAL / 2.0
                    {
                        let origin = text_anchorpoint - vec2(0.0, text_input.scale / 2.0);
                        let cursor_pos: Vec2 = if text_input.text.is_empty() {
                            origin
                        } else if text_input.cursor >= text_input.text.len() {
                            let bounds: Vec2 = text.measure(ctx)?.into();
                            origin + vec2(bounds.x, 0.0)
                        } else {
                            let glyph_positions = text.glyph_positions(ctx)?;
                            origin + vec2(glyph_positions[text_input.cursor].x, 0.0)
                        };
                        Mesh::new_line(
                            ctx,
                            &[cursor_pos, cursor_pos + vec2(0.0, text_input.scale)],
                            2.0,
                            Color::BLACK,
                        )?
                        .draw(canvas);
                    }
                }
            }
        }
        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let res: Vec2 = ctx.gfx.drawable_size().into();
        self.mouse_position = ctx.mouse.position().into();
        self.cursor_override = None;
        let mouse_pressed = ctx.mouse.button_just_pressed(MouseButton::Left);
        let just_pressed_keys: HashSet<_> = ctx
            .keyboard
            .pressed_logical_keys
            .iter()
            .filter(|key| !self.last_pressed_keys.contains(key))
            .cloned()
            .collect();
        self.last_pressed_keys = ctx.keyboard.pressed_logical_keys.clone();
        for element in self.elements.iter() {
            match element {
                UIElement::Button(button) => {
                    let button = button.borrow();
                    if button.state != UIElementState::Enabled {
                        continue;
                    }

                    let bounds = button.bounds.corrected_bounds(res);
                    if bounds.contains(self.mouse_position) {
                        self.cursor_override = Some(CursorIcon::Pointer);
                        if ctx.mouse.button_just_released(MouseButton::Left) {
                            self.event_sender.send(button.event.clone().into()).unwrap();
                        }
                    }
                }
                UIElement::TextInput(text_input) => {
                    let mut text_input = text_input.borrow_mut();
                    if text_input.state != UIElementState::Enabled {
                        continue;
                    }

                    let bounds = text_input.bounds.corrected_bounds(res);
                    if bounds.contains(self.mouse_position) {
                        self.cursor_override = Some(CursorIcon::Text);
                        if mouse_pressed {
                            text_input.focused = true;
                            let (text, anchorpoint) = text_input.get_drawable_text(ctx);
                            let text_bounds: Vec2 = text.measure(ctx)?.into();
                            text_input.cursor = text
                                .glyph_positions(ctx)?
                                .iter()
                                .cloned()
                                .map(Vec2::from)
                                .chain([text_bounds])
                                .enumerate()
                                .min_by_f32_key(|(_, pos)| {
                                    ((*pos + anchorpoint) - self.mouse_position).x.abs()
                                })
                                .map_or(0, |(i, _)| i)
                        }
                    } else if mouse_pressed {
                        text_input.focused = false;
                    }

                    if text_input.focused {
                        let additional_keys = if ctx.keyboard.is_key_repeated() {
                            &ctx.keyboard.pressed_logical_keys
                        } else {
                            &HashSet::new()
                        };
                        for key in just_pressed_keys.iter().chain(additional_keys) {
                            log::trace!("key = {key:?}");
                            match key {
                                Key::Named(NamedKey::Delete) => text_input.delete_char(),
                                Key::Named(NamedKey::Backspace) => text_input.backspace_char(),
                                Key::Named(NamedKey::ArrowRight) => text_input.right(),
                                Key::Named(NamedKey::ArrowLeft) => text_input.left(),
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
                                            text_input.type_char(chr);
                                        }
                                    } else {
                                        for c in ch.chars() {
                                            text_input.type_char(c);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if let Some(cursor_icon) = self.cursor_override {
            set_cursor_type(ctx, cursor_icon);
        }
        Ok(())
    }
}
