#version 460


layout(location=0) out vec2 v_tex_coords;

void main() {
    vec2 uv = vec2((gl_VertexIndex << 1) & 2, gl_VertexIndex & 2);
	gl_Position = vec4(uv * vec2(2, -2) + vec2(-1, 1), 0, 1);
    v_tex_coords = uv;
}
