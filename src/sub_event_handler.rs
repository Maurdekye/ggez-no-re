use std::error::Error;

use ggez::{
    Context, GameError,
    context::{ContextFields, Has, HasMut},
    event::EventHandler,
    graphics::{Canvas, Color, GraphicsContext},
    input::mouse::MouseContext,
};

pub trait SubEventHandler<C = Context, E = GameError> {
    fn update(&mut self, ctx: &mut C) -> Result<(), E>;
    fn draw(&mut self, ctx: &mut C, canvas: &mut Canvas) -> Result<(), E>;
    fn quit_event(&mut self, _ctx: &mut C) -> Result<bool, E> {
        Ok(false)
    }
    fn mouse_wheel_event(&mut self, _ctx: &mut C, _x: f32, _y: f32) -> Result<(), E> {
        Ok(())
    }
    fn event_handler(self) -> EventHandlerWrapper<Self>
    where
        Self: Sized,
    {
        EventHandlerWrapper(self)
    }
}

pub struct EventHandlerWrapper<H>(H);

impl<S, C, E> EventHandler<C, E> for EventHandlerWrapper<S>
where
    C: Has<GraphicsContext>
        + HasMut<GraphicsContext>
        + HasMut<MouseContext>
        + HasMut<ContextFields>,
    S: SubEventHandler<C, E>,
    E: Error + From<GameError>,
{
    fn update(&mut self, ctx: &mut C) -> Result<(), E> {
        self.0.update(ctx)
    }

    fn draw(&mut self, ctx: &mut C) -> Result<(), E> {
        let mut canvas = Canvas::from_frame(ctx, Color::WHITE);
        self.0.draw(ctx, &mut canvas)?;
        Ok(canvas.finish(ctx)?)
    }

    fn quit_event(&mut self, ctx: &mut C) -> Result<bool, E> {
        self.0.quit_event(ctx)
    }

    fn mouse_wheel_event(&mut self, ctx: &mut C, x: f32, y: f32) -> Result<(), E> {
        self.0.mouse_wheel_event(ctx, x, y)
    }
}

pub trait EventReceiver<E = GameError> {
    type Event;

    fn handle_event(&mut self, ctx: &mut Context, event: Self::Event) -> Result<(), E>;
    fn poll_event(&mut self) -> Result<Option<Self::Event>, E>;
    fn handle_events(&mut self, ctx: &mut Context) -> Result<(), E> {
        while let Some(event) = self.poll_event()? {
            self.handle_event(ctx, event)?;
        }
        Ok(())
    }
}
