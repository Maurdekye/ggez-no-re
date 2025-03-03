use std::sync::mpsc::Sender;

use ggez::{
    Context, GameResult,
    glam::Vec2,
    graphics::{Canvas, Color, DrawMode, DrawParam, Mesh, Rect, Text},
    input::mouse::CursorIcon,
    winit::event::MouseButton,
};

use crate::util::{ContextExt, DrawableWihParamsExt, color_mul};

use super::{BUTTON_COLOR, Bounds, UIElementState};
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

    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas, mouse: Vec2) -> GameResult<()> {
        if self.state == UIElementState::Invisible {
            return Ok(());
        }

        let bounds = self.bounds.corrected_bounds(ctx.res());
        let contains = bounds.contains(mouse);
        let color = match (
            &self.state,
            contains,
            ctx.mouse.button_pressed(MouseButton::Left),
        ) {
            (UIElementState::Disabled, _, _) => <[f32; 4]>::from(self.color)
                .map(|x| (x - 0.5) * 0.25 + 0.5)
                .into(),
            (_, true, true) => color_mul(self.color, 0.8),
            (_, true, _) => color_mul(self.color, 1.2),
            _ => self.color,
        };
        Mesh::new_rounded_rectangle(ctx, DrawMode::fill(), bounds, 5.0, color)?.draw(canvas);
        self.text
            .with_params(self.text_drawparam)
            .centered_on(ctx, bounds.center().into())?
            .draw(canvas);
        Ok(())
    }

    pub fn update<T>(
        &mut self,
        ctx: &Context,
        mouse: Vec2,
        event_sender: &Sender<T>,
    ) -> GameResult<Option<CursorIcon>>
    where
        E: Clone,
        T: From<E>,
    {
        if self.state != UIElementState::Enabled {
            return Ok(None);
        }

        if self.bounds.corrected_bounds(ctx.res()).contains(mouse) {
            if ctx.mouse.button_just_released(MouseButton::Left) {
                event_sender.send(self.event.clone().into()).unwrap();
            }
            return Ok(Some(CursorIcon::Pointer));
        }
        Ok(None)
    }
}
