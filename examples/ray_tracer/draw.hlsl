RWTexture2D<unorm float4> to_draw_texture;
RWTexture2D<uint4> depth;

layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

void main()
{
    uvec3 global_invocation_id = gl_GlobalInvocationID;
    uint x = global_invocation_id.x;
    uint y = global_invocation_id.y;
    ivec2 pos = ivec2(x, y);

    to_draw_texture[pos] = depth.load(pos) / 50.0;
}
