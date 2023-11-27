@group(0) @binding(0)
var to_draw_texture: texture_storage_2d<f32, write>;
@group(0) @binding(1)
var<uniform> gpu_data: GpuData;

struct GpuData{
    f32 time;
    u32 width;
    u32 height;
}

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    u32 x = global_invocation_id.x;
    u32 y = global_invocation_id.y;
    Vec3<i32> pos = Vec3<i32>(x,y);

    f32 val = sin(float(x * 5) / width + time) / 2 + 0.5; // calulate sin value at certain x
    bool color = val * height < y + 1 && val * height > y - 1;   // the pixel has to be colored if it is at most 1 pixel away from the sin value
    
    textureStore(to_draw_texture, pos, vec4(color, 0, 0, 1));
    return;
}