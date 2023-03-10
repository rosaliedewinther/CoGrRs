use bytemuck::Pod;

use crate::CoGrReadHandle;

pub struct WGPUReadhandle(u32);

impl CoGrReadHandle for WGPUReadhandle {
    fn wait_and_read<'a, T: Pod>(self, _gpu_context: &crate::Renderer) -> &'a [T] {
        todo!()
    }
}
