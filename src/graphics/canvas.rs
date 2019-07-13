use std::rc::Rc;

use crate::glm::Mat4;
use crate::graphics::opengl::GLFramebuffer;
use crate::graphics::{DrawParams, Drawable, FilterMode, Texture};
use crate::Context;

/// A 2D texture that can be used for off-screen rendering.
///
/// This is sometimes referred to as a 'render texture' or 'render target' in other
/// frameworks.
///
/// Canvases can be useful if you want to do some rendering upfront and then cache the result
/// (e.g. a static background), or if you want to apply transformations/shaders to multiple
/// things simultaneously.
#[derive(Debug, Clone, PartialEq)]
pub struct Canvas {
    pub(crate) texture: Texture,
    pub(crate) framebuffer: Rc<GLFramebuffer>,
    pub(crate) projection: Mat4,
}

impl Canvas {
    /// Creates a new canvas.
    pub fn new(ctx: &mut Context, width: i32, height: i32) -> Canvas {
        // TODO: Make this return Result in 0.3
        ctx.gl
            .new_canvas(width, height, true)
            .expect("Could not create canvas")
    }

    /// Returns the width of the canvas.
    pub fn width(&self) -> i32 {
        self.texture.width()
    }

    /// Returns the height of the canvas.
    pub fn height(&self) -> i32 {
        self.texture.height()
    }

    /// Returns the filter mode being used by the canvas.
    pub fn filter_mode(&self) -> FilterMode {
        self.texture.filter_mode()
    }

    /// Sets the filter mode that should be used by the canvas.
    pub fn set_filter_mode(&mut self, ctx: &mut Context, filter_mode: FilterMode) {
        self.texture.set_filter_mode(ctx, filter_mode);
    }

    /// Returns the canvas' underlying texture.
    pub fn texture(&self) -> &Texture {
        &self.texture
    }
}

impl Drawable for Canvas {
    fn draw<P>(&self, ctx: &mut Context, params: P)
    where
        P: Into<DrawParams>,
    {
        self.texture.draw(ctx, params)
    }
}
