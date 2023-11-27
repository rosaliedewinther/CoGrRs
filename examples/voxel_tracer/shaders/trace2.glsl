layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(rgba32f) uniform image2D primary_ray_direction;
layout(rgba16f) uniform image2D screen_texture;
buffer trace_data{
    vec3 scatteringCoefficients;
    float coeiff;
    vec3 camera_pos;
    float time;
    uvec2 screen_dimensions;
};


vec3 sunPosition = vec3(cos(time)*100000, sin(time)*100000,0);    // Sun position in world space

const float PI = 3.141592653589793;

// Function to calculate the Rayleigh phase function
float rayleighPhase(float cosTheta) {
    return 3.0 / (16.0 * PI) * (1.0 + cosTheta * cosTheta);
}

void main() {
     ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    vec3 RayDir = imageLoad(primary_ray_direction, pos).xyz;
    // Calculate the direction to the sun
    vec3 sunDir = normalize(sunPosition - camera_pos);

    // Calculate the angle between the ray direction and the sun direction
    float cosAngle = dot(normalize(RayDir), sunDir);

    // Calculate the Rayleigh scattering term
    vec3 rayleigh = exp(-scatteringCoefficients * 0.0015) * rayleighPhase(cosAngle);

    // Calculate the Mie scattering term (simplified for demonstration)
    float mie = exp(-scatteringCoefficients.y * 0.0015) * 0.1;

    // Calculate the color of the sky
    vec3 skyColor = vec3(0.7, 0.8, 1.0);  // Gradient color for the sky
    vec3 finalColor = skyColor * (rayleigh + mie);

    // Add the sun color based on the angle between the ray direction and the sun direction
    float sunIntensity = max(0.0, dot(RayDir, -sunDir));
    vec3 sunColor = vec3(1.0, 0.8, 0.6);  // Sun color
    finalColor += sunColor * pow(sunIntensity, 8.0);

    // Apply gamma correction
    finalColor = pow(finalColor, vec3(1.0 / 2.2));

    imageStore(screen_texture, pos, vec4(finalColor, 1));
}