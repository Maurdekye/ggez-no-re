use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    sync::{Mutex, mpsc::Sender},
};

use button::Button;
use checkbox::Checkbox;
use ggez::{
    Context, GameError, GameResult,
    glam::{Vec2, vec2},
    graphics::{Canvas, Color, Rect, Text},
    input::mouse::{CursorIcon, set_cursor_type},
    winit::keyboard::Key,
};
use text_input::TextInput;

use crate::{
    sub_event_handler::SubEventHandler,
    util::{AnchorPoint, RectExt, TextExt},
};

pub mod button;
pub mod checkbox;
pub mod text_input;

static CURSOR_ICON_BUFFER: Mutex<CursorIcon> = Mutex::new(CursorIcon::Default);
static CURSOR_ICON: Mutex<CursorIcon> = Mutex::new(CursorIcon::Default);

pub fn set_cursor_icon(icon: CursorIcon) {
    *CURSOR_ICON_BUFFER.lock().unwrap() = icon;
}

pub fn flush_cursor_icon(ctx: &mut Context) {
    let new_icon = *CURSOR_ICON_BUFFER.lock().unwrap();
    let mut cursor_icon = CURSOR_ICON.lock().unwrap();
    if new_icon != *cursor_icon {
        *cursor_icon = new_icon;
        set_cursor_type(ctx, new_icon);
    }
}

pub fn begin_context(_ctx: &mut Context) {
    set_cursor_icon(CursorIcon::Default);
}

pub fn end_context(ctx: &mut Context) {
    flush_cursor_icon(ctx);
}

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

#[derive(Clone)]
pub enum UIElement<B, T, C> {
    Button(B),
    TextInput(T),
    Checkbox(C),
}

impl<B, T, C> UIElement<B, T, C> {
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

pub struct UIManager<E = (), T = E> {
    #[allow(clippy::type_complexity)]
    elements: Vec<UIElement<Rc<RefCell<Button<E>>>, Rc<RefCell<TextInput>>, Rc<RefCell<Checkbox>>>>,
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
        elements: [UIElement<Button<E>, TextInput, Checkbox>; N],
    ) -> (
        UIManager<E, T>,
        [UIElement<Rc<RefCell<Button<E>>>, Rc<RefCell<TextInput>>, Rc<RefCell<Checkbox>>>; N],
    ) {
        let return_elements = elements.map(|elem| match elem {
            UIElement::Button(button) => UIElement::Button(Rc::new(RefCell::new(button))),
            UIElement::TextInput(text_input) => {
                UIElement::TextInput(Rc::new(RefCell::new(text_input)))
            }
            UIElement::Checkbox(checkbox) => UIElement::Checkbox(Rc::new(RefCell::new(checkbox))),
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
        elements: [UIElement<Button<E>, TextInput, Checkbox>; N],
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
        for element in self.elements.iter() {
            match element {
                UIElement::Button(button) => {
                    button.borrow().draw(ctx, canvas, self.mouse_position)?
                }
                UIElement::TextInput(text_input) => {
                    text_input.borrow().draw(ctx, canvas, self.mouse_position)?
                }
                UIElement::Checkbox(checkbox) => {
                    checkbox.borrow().draw(ctx, canvas, self.mouse_position)?
                }
            }
        }
        Ok(())
    }

    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        self.mouse_position = ctx.mouse.position().into();
        self.cursor_override = None;
        let just_pressed_keys: HashSet<_> = ctx
            .keyboard
            .pressed_logical_keys
            .iter()
            .filter(|key| !self.last_pressed_keys.contains(key))
            .cloned()
            .collect();
        self.last_pressed_keys = ctx.keyboard.pressed_logical_keys.clone();
        for element in self.elements.iter() {
            let new_override = match element {
                UIElement::Button(button) => {
                    button
                        .borrow_mut()
                        .update(ctx, self.mouse_position, &self.event_sender)?
                }
                UIElement::TextInput(text_input) => {
                    text_input
                        .borrow_mut()
                        .update(ctx, self.mouse_position, &just_pressed_keys)?
                }
                UIElement::Checkbox(checkbox) => {
                    checkbox.borrow_mut().update(ctx, self.mouse_position)?
                }
            };
            if let Some(new_override) = new_override {
                self.cursor_override = Some(new_override);
            }
        }

        if let Some(cursor_icon) = self.cursor_override {
            set_cursor_icon(cursor_icon);
        }
        Ok(())
    }
}

pub trait UIElementRenderable {
    fn get_corrected_bounds(&self, ctx: &Context) -> Rect;
    fn get_state(&self) -> UIElementState;

    fn render_label(
        &self,
        ctx: &Context,
        canvas: &mut Canvas,
        text: &Text,
        anchor_point: AnchorPoint,
    ) -> GameResult<()> {
        if self.get_state() == UIElementState::Invisible {
            return Ok(());
        }

        let (rel_offset, abs_offset, text_anchor) = match anchor_point {
            AnchorPoint::CenterWest => (vec2(0.0, 0.5), vec2(-6.0, 0.0), AnchorPoint::CenterEast),
            AnchorPoint::CenterEast => (vec2(1.0, 0.5), vec2(6.0, 0.0), AnchorPoint::CenterWest),
            AnchorPoint::NorthCenter => (vec2(0.5, 0.0), vec2(0.0, -6.0), AnchorPoint::SouthCenter),
            anchor_point => unimplemented!("Anchor point type {anchor_point:?} is unimplemented"),
        };
        text.anchored_by(
            ctx,
            self.get_corrected_bounds(ctx).parametric(rel_offset) + abs_offset,
            text_anchor,
        )?
        .color(Color::BLACK)
        .draw(canvas);

        Ok(())
    }
}
