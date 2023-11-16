layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;

layout(rgba8) uniform image2D to_draw_texture;
buffer gpu_data
{
    float time;
    uint width;
    uint height;
};

void main() {
    uvec3 global_invocation_id = gl_GlobalInvocationID;
    uint x = global_invocation_id.x;
    uint y = global_invocation_id.y;
    ivec2 pos = ivec2(x,y);

    float val = sin(float(x * 5) / width + time) / 2 + 0.5; // calulate sin value at certain x
    bool color = val * height < y + 1 && val * height > y - 1;   // the pixel has to be colored if it is at most 1 pixel away from the sin value
    
    imageStore(to_draw_texture, pos, vec4(color, 0, 0, 1));
    return;
}