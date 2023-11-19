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

const float atmosphereRadius = 1500;
const float planetRadius = 800;
const float sunRadius = 0.99;

vec2 intersect(vec3 origin, vec3 direction, vec3 center, float radius2) {
    float t0, t1; // solutions for t if the ray intersects
    // geometric solution
    vec3 L = center - origin;
    float tca = dot(L, direction);
    // if (tca < 0) return false;
    float d2 = dot(L, L) - tca * tca;
    if (d2 > radius2) return vec2(-1,-1);
    float thc = sqrt(radius2 - d2);
    t0 = tca - thc;
    t1 = tca + thc;
    if (t0 > t1) {
        float temp = t0;
        t0 = t1;
        t1 = temp;
    }

    return vec2(t0, t1);
}

vec3 mie(float dist, vec3 sunL){
    return max(exp(-pow(dist, 0.25)) * sunL - 0.4, 0.0);
}

float getSkyDensity(vec3 position){
    // distance from ground` 
    return (1-exp(-((dot(position, position) - planetRadius)/(atmosphereRadius-planetRadius))));
}

float stepSize(inout vec3 origin, vec3 direction, vec2 intersection_results, uint steps){
    if (intersection_results.x >= 0){
        origin += direction*(intersection_results.x+0.0001);
    }
    return (intersection_results.y - max(intersection_results.x, 0.0)) / steps;
}

const uint steps = 32;
const uint shadow_steps = 8;

float getSunOpticalDepth(vec3 origin, vec3 sunDir){
    float opticalDepth = 0;
    vec2 intersection_results = intersect(origin, sunDir, vec3(0), atmosphereRadius);
    float step_size = stepSize(origin, sunDir, intersection_results, shadow_steps);
    for (int i = 1; i < shadow_steps; i++){
        float density = getSkyDensity(origin + i*step_size*sunDir);
        opticalDepth += density * step_size;
    }
    return opticalDepth;
}

float getSun(vec3 direction, vec3 sunDir){
    if (sunRadius < dot(direction, sunDir)){
        return 1.0;
    } else {
        return 0.0;
    }
}

vec3 getSky(vec3 origin, vec3 direction){
    vec3 sundir = normalize( vec3(cos(iTime/50), sin(iTime/50), 0.) );
    vec2 intersection_results = intersect(origin, direction, vec3(0), atmosphereRadius);
    float step_size = stepSize(origin, direction, intersection_results, steps);
    float opticalDepth = 0;
    vec3 light = vec3(0,0,0);
    
    if (intersection_results.y >= 0){
        for (int i = 1; i < steps; i++){
            vec3 samplePos = origin + direction * step_size * i;
            float density = getSkyDensity(samplePos);
            opticalDepth += density * step_size;
            float sunOpticalDepth = getSunOpticalDepth(samplePos, sundir);
            light += density*density * exp(-(opticalDepth + sunOpticalDepth))*skylight;
        }
    }

    light += vec3(1) * getSun(direction, sundir) * exp(-opticalDepth); 
    
    
    return vec3(light);
}



void main()
{
    ivec2 pos = ivec2(gl_GlobalInvocationID.xy);
    vec3 direction = imageLoad(primary_ray_direction, pos).xyz;
    vec4 color;
    vec3 ball_pos = vec3(0,0,0);
    vec2 intersection_results = intersect(camera_pos, direction, ball_pos, planetRadius);
    if (intersection_results.x >= 0 && intersection_results.y >= 0){
        vec3 hit_pos = camera_pos + intersection_results.x*direction;
        vec3 new_dir = reflect(direction, normalize(hit_pos - ball_pos));
        color = vec4(getSky(hit_pos, new_dir)* vec3(1,1,0.5), 1.0);
    } else {
        color = vec4(getSky(camera_pos, direction), 1.0);
    }

	//color = color / (2.0 * color + 0.5 - color);
    imageStore(screen_texture, pos, color);
}