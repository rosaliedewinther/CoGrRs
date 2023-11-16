layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(rgba32f) uniform image2D ray_direction_out;
buffer camera
{
    vec3 position;
    float aperture;
    vec3 direction;
    float focal_length;
    vec3 direction_side;
    float sensor_height;
    vec3 direction_up;
    uint random_seed;
    uvec2 screen_dimensions;
};

vec3 to_world_space(vec2 shift){
    vec3 position_horizontal = shift.x * direction_side;
    vec3 position_vertical = shift.y * direction_up;
    return position_horizontal + position_vertical;
}

uint wang_hash(uint seed) {
    seed = (seed ^ 61) ^ (seed >> 16);
    seed *= 9;
    seed = seed ^ (seed >> 4);
    seed *= 0x27d4eb2d;
    seed = seed ^ (seed >> 15);
    return seed;
}

// Algorithm "xor" from p. 4 of Marsaglia, "Xorshift RNGs"
uint random_uint(inout uint state) {
  	uint x = state;
  	x ^= x << 13;
  	x ^= x >> 17;
  	x ^= x << 5;
  	return state = x;
}

float random_float(inout uint state) {
  	return random_uint(state) * 2.3283064365387e-10f;
}

void main()
{
    uint x = gl_GlobalInvocationID.x;
    uint y = gl_GlobalInvocationID.y;
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);

    uint screen_width = screen_dimensions.x;
    uint screen_height = screen_dimensions.y;
    uint random_state = wang_hash((1 + x + y * screen_width) * random_seed);

    vec3 sensor_center = position - direction * focal_length;
    float horizontal_shift = ((float(x)/screen_width)-0.5) * sensor_height * (screen_width/screen_height);
    float vertical_shift = ((float(y)/screen_height)-0.5) * sensor_height;
    vec3 position_on_sensor = sensor_center + to_world_space(vec2(horizontal_shift, vertical_shift));

    //vec2 pinhole_offset = random_point_circle(random_state) * (focal_length/aperture); 
    vec3 pinhole_passthrough_position = position;// + to_world_space(pinhole_offset);

    vec3 ray_direction = pinhole_passthrough_position - position_on_sensor;
    imageStore(ray_direction_out, pos, vec4(ray_direction, 0));
}