use crevice::std140::AsStd140;
use ggez::{
    Context, GameError, GameResult,
    graphics::{
        Canvas, Color, DrawMode, Mesh, Rect, Shader, ShaderBuilder, ShaderParams,
        ShaderParamsBuilder,
    },
};

use crate::{
    sub_event_handler::SubEventHandler,
    util::{ContextExt, DrawableWihParamsExt},
};

#[macro_export]
macro_rules! build_shader {
    ($ctx:expr, $src:literal, $uniforms:expr) => {
        $crate::shader_scene::ShaderScene::build($ctx, include_str!($src), $uniforms)
    };
    ($ctx:expr, $src:literal) => {
        $crate::shader_scene::ShaderScene::build($ctx, include_str!($src), Default::default())
    };
}

pub struct ShaderScene<C>
where
    C: AsStd140,
{
    pub uniforms: C,
    shader: Shader,
    params: ShaderParams<C>,
}

impl<C> ShaderScene<C>
where
    C: AsStd140,
{
    pub fn build(ctx: &mut Context, src: &str, uniforms: C) -> GameResult<ShaderScene<C>> {
        let params = ShaderParamsBuilder::new(&uniforms).build(ctx);
        let shader = ShaderBuilder::new().fragment_code(src).build(ctx)?;
        Ok(ShaderScene {
            uniforms,
            shader,
            params,
        })
    }
}

impl<C> SubEventHandler for ShaderScene<C>
where
    C: AsStd140,
{
    fn update(&mut self, _ctx: &mut Context) -> Result<(), GameError> {
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> Result<(), GameError> {
        let res = ctx.res();
        self.params.set_uniforms(ctx, &self.uniforms);
        canvas.set_shader(&self.shader);
        canvas.set_shader_params(&self.params);
        Mesh::new_rectangle(
            ctx,
            DrawMode::fill(),
            Rect::new(0.0, 0.0, res.x, res.y),
            Color::WHITE,
        )?
        .draw(canvas);
        canvas.set_default_shader();
        Ok(())
    }
}
