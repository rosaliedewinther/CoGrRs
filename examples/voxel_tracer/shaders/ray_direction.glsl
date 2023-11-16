layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(rgba32f) uniform image2D primary_ray_direction;
layout(rgba16f) uniform image2D screen_texture;

void main()
{
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    vec4 direction = imageLoad(primary_ray_direction, pos);
    imageStore(screen_texture, pos, (direction+1)/2);
}