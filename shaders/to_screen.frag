#version 460

layout(location=0) in vec2 v_tex_coords;
layout(location=0) out vec4 f_color;

layout(rgba8, binding = 0) readonly uniform image2D to_draw;

void main() {
    vec2 size = imageSize(to_draw);
    f_color = vec4(imageLoad(to_draw, ivec2(v_tex_coords*size)));
}