layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(rgba32f) uniform image2D primary_ray_direction;
layout(rgba16f) uniform image2D screen_texture;
buffer trace_data{
    vec3 skylight;
    float coeiff;
    vec3 camera_pos;
    float iTime;
    uvec2 screen_dimensions;
};

bool intersect(vec3 origin, vec3 direction, vec3 center, float radius2, out float t) {
        float t0, t1; // solutions for t if the ray intersects
        // geometric solution
        vec3 L = center - origin;
        float tca = dot(L, direction);
        // if (tca < 0) return false;
        float d2 = dot(L, L) - tca * tca;
        if (d2 > radius2) return false;
        float thc = sqrt(radius2 - d2);
        t0 = tca - thc;
        t1 = tca + thc;
        if (t0 > t1) {
            float temp = t0;
            t0 = t1;
            t1 = temp;
        }


        if (t0 < 0) {
            t0 = t1; // if t0 is negative, let's use t1 instead
            if (t0 < 0) return false; // both t0 and t1 are negative
        }

        t = t0;

        return true;
}

vec3 mie(float dist, vec3 sunL){
    return max(exp(-pow(dist, 0.25)) * sunL - 0.4, 0.0);
}

vec3 getSky(vec3 rayDir){
    vec3 sundir = normalize( vec3(cos(iTime), sin(iTime), 0.) );
    
    float yd = min(rayDir.y, 0.);
    rayDir.y = max(rayDir.y, 0.);
    
    vec3 col = vec3(0.);
    
    col += vec3(.4, .4 - exp( -rayDir.y*20. )*.15, .0) * exp(-rayDir.y*9.); // Red / Green 
    col += vec3(.3, .5, .6) * (1. - exp(-rayDir.y*8.) ) * exp(-rayDir.y*.9) ; // Blue
    
    col = mix(col*1.2, vec3(.3),  1.-exp(yd*100.)); // Fog
    
    col += vec3(1.0, .8, .55) * pow( max(dot(rayDir,sundir),0.), 15. ) * .6; // Sun
    col += pow(max(dot(rayDir, sundir),0.), 150.0) *.15;
    
    return col;
}

void main()
{
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    vec3 direction = imageLoad(primary_ray_direction, pos).xyz;
    vec4 color;
    float t;
    vec3 ball_pos = vec3(1,0,0);
    if (intersect(camera_pos, direction, ball_pos, 1.0, t)){
        vec3 hit_pos = camera_pos + t*direction;
        vec3 new_dir = reflect(direction, normalize(hit_pos - ball_pos));
        color = vec4(getSky(new_dir)* vec3(1,1,0.5), 1.0);
    } else {
        color = vec4(getSky(direction), 1.0);
    }

	color = color / (2.0 * color + 0.5 - color);
    imageStore(screen_texture, pos, color);
}