use wgpu::{CommandEncoder, SurfaceTexture, TextureView};

use crate::{Context};

pub struct AutoEncoder<'a> {
    pub encoder: Option<CommandEncoder>,
    pub gpu_context: &'a mut Context,
    pub surface_texture: Option<SurfaceTexture>,
    pub surface_texture_view: Option<TextureView>,
}

impl<'a> AutoEncoder<'a> {
    pub fn new(
        encoder: CommandEncoder,
        gpu_context: &'a mut Context,
        surface_texture: Option<SurfaceTexture>,
        surface_texture_view: Option<TextureView>,
    ) -> Self {
        AutoEncoder {
            encoder: Some(encoder),
            gpu_context,
            surface_texture,
            surface_texture_view,
        }
    }
}

impl<'a> Drop for AutoEncoder<'a> {
    fn drop(&mut self) {
        match (&mut self.encoder, &mut self.surface_texture, &self.surface_texture_view) {
            (encoder, surface_texture, surface_texture_view) if encoder.is_some() && surface_texture.is_some() && surface_texture_view.is_some() => {
                self.gpu_context
                    .queue
                    .submit(std::iter::once(encoder.take().unwrap().finish()));
                let surface = surface_texture.take().unwrap();
                surface.present();
            }
            (encoder, surface_texture, _) if encoder.is_some() && surface_texture.is_none() => {
                self.gpu_context
                    .queue
                    .submit(std::iter::once(encoder.take().unwrap().finish()));
            }
            (encoder, _, _) if encoder.is_none() => {
                panic!("encoder is none while that should be impossbile");
            }
            (_, _, _) => panic!("impossible state"),
        }
    }
}
