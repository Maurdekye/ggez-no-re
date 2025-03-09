use ggez::{
    Context, GameResult,
    glam::{Vec2, vec2},
    graphics::{Canvas, Color, DrawMode, Mesh},
    input::mouse::CursorIcon,
    winit::event::MouseButton,
};

use crate::util::{ContextExt, DrawableWihParamsExt, refit_to_rect};

use super::{Bounds, UIElementRenderable, UIElementState, TEXTINPUT_BODY, TEXTINPUT_BORDER};

#[derive(Debug)]
pub struct Checkbox {
    pub bounds: Bounds,
    pub state: UIElementState,
    pub checked: bool,
}

impl Checkbox {
    pub fn new(bounds: Bounds) -> Checkbox {
        Checkbox {
            bounds,
            checked: false,
            state: UIElementState::Enabled,
        }
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

        if self.checked {
            Mesh::new_polygon(
                ctx,
                DrawMode::fill(),
                &[
                    vec2(0.25, 0.33),
                    vec2(0.5, 0.585),
                    vec2(1.05, 0.03),
                    vec2(1.17, 0.15),
                    vec2(0.5, 0.825),
                    vec2(0.13, 0.45),
                ]
                .map(|pos| refit_to_rect(pos, bounds)),
                Color::BLACK,
            )?
            .draw(canvas);
        }

        Ok(())
    }

    pub fn update(&mut self, ctx: &Context, mouse: Vec2) -> GameResult<Option<CursorIcon>> {
        if self.state != UIElementState::Enabled {
            return Ok(None);
        }

        if self.bounds.corrected_bounds(ctx.res()).contains(mouse) {
            if ctx.mouse.button_just_released(MouseButton::Left) {
                self.checked = !self.checked;
            }
            return Ok(Some(CursorIcon::Pointer));
        }
        Ok(None)
    }
}

impl UIElementRenderable for Checkbox {
    fn get_corrected_bounds(&self, ctx: &Context) -> ggez::graphics::Rect {
        self.bounds.corrected_bounds(ctx.res())
    }

    fn get_state(&self) -> UIElementState {
        self.state
    }
}