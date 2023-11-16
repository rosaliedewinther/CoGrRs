struct BVHNode{
    float minx;
    float miny;
    float minz;
    float maxx;
    float maxy;
    float maxz;
    int left_first;
    int count;
};

struct StackNode{
    uint bvh_node_index;
    float dist;
};

struct Triangle{
    vec3 p1;
    float pad1;
    vec3 p2;
    float pad2;
    vec3 p3;
    float pad3;
};

layout(rgba8) uniform image2D to_draw_texture;
layout(std430) buffer triangles_block { Triangle triangles[]; };
buffer bvh_nodes_block { BVHNode bvh_nodes[]; };
buffer gpu_data
{
    vec3 camera_dir;
    float width;
    vec3 camera_pos;
    float height;
    vec3 camera_side;
    float half_width;
    vec3 camera_up;
    float half_height;
    float time;
    uint padding1;
    uint padding2;
    uint padding3;
};
layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;



#define FLT_MAX 3.402823466e+38

vec3 my_cross(vec3 a, vec3 b){
    return vec3(  
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x
    );
}


void intersect_ray_triangle(
    in vec3 ray_o, 
    in vec3 ray_dir, 
    inout float ray_t, 
    inout uint prim_id,
    in uint triangle_id
){
    vec3 p1 = triangles[triangle_id].p1;
    vec3 p2 = triangles[triangle_id].p2;
    vec3 p3 = triangles[triangle_id].p3;
    vec3 p1_to_p2 = p2 - p1;
    vec3 p1_to_p3 = p3 - p1;
    vec3 uvec = my_cross(ray_dir, p1_to_p3);
    float det = dot(p1_to_p2, uvec);
    float inv_det = 1/det;
    vec3 a_to_origin = ray_o - p1;
    float u = dot(a_to_origin, uvec) * inv_det;
    if (u < 0 || u > 1){
        return;
    }
    vec3 vvec = my_cross(a_to_origin, p1_to_p2);
    float v = dot(ray_dir, vvec) * inv_det;
    if (v < 0 || u + v > 1){
        return;
    }
    float dist = dot(p1_to_p3, vvec) * inv_det;
    if (dist > 0.00000001 && dist < ray_t){
        ray_t = dist;
        prim_id = triangle_id;
    }
}

float intersect_ray_aabb(
    in vec3 ray_o, 
    in vec3 ray_dir, 
    in vec3 ray_dirr, 
    inout float ray_t, 
    inout uint prim_id,
    in uint aabb_id
){
    BVHNode node = bvh_nodes[aabb_id];
    vec3 v_max = vec3(node.maxx,node.maxy, node.maxz); 
    vec3 v_min = vec3(node.minx, node.miny, node.minz); 
    vec3 tMin  = (v_min - ray_o)*ray_dirr;
    vec3 tMax = (v_max - ray_o)*ray_dirr;
    vec3 t1 = min(tMin, tMax);
    vec3 t2 = max(tMin, tMax);
    float tNear = max(max(t1.x, t1.y), t1.z);
    float tFar = min(min(t2.x, t2.y), t2.z);
    if (tFar >= tNear && tNear < ray_t && tFar > 0){
        return tNear;
    } else {
        return FLT_MAX;
    }
}

void traverse(
    in vec3 ray_o, 
    in vec3 ray_dir, 
    in vec3 ray_dirr, 
    inout float ray_t, 
    inout uint prim_id
){
    StackNode stack[32];
    uint node_index = 0;
    uint stack_ptr = 0;
    while(true){
        if (bvh_nodes[node_index].count > 0){
            for (int i = 0; i < bvh_nodes[node_index].count; i++){
                intersect_ray_triangle(
                    ray_o, 
                    ray_dir, 
                    ray_t, 
                    prim_id,
                    bvh_nodes[node_index].left_first + i
                );
            }
            if (stack_ptr == 0){
                break;
            } else {
                float t = FLT_MAX;
                while (t >= ray_t){
                    if (stack_ptr == 0){
                        return;
                    }
                    stack_ptr-=1;
                    StackNode sn = stack[stack_ptr];
                    t = sn.dist;
                    node_index = sn.bvh_node_index;
                }
                continue;
            }
        }
        uint child1 = bvh_nodes[node_index].left_first;
        uint child2 = bvh_nodes[node_index].left_first + 1;

        float dist1 = intersect_ray_aabb(
            ray_o, 
            ray_dir, 
            ray_dirr, 
            ray_t, 
            prim_id,
            child1
        );
        float dist2 = intersect_ray_aabb(
            ray_o, 
            ray_dir, 
            ray_dirr, 
            ray_t, 
            prim_id,
            child2
        );
        if (dist1 > dist2){
            uint tempu = child1;
            child1 = child2;
            child2 = tempu;
            float tempf = dist1;
            dist1 = dist2;
            dist2 = tempf;
        }
        if (dist1 == FLT_MAX){
            if (stack_ptr == 0){
                return;
            } else {
                float t = FLT_MAX;
                while (t >= ray_t){
                    if (stack_ptr == 0){
                        return;
                    }
                    stack_ptr-=1;
                    StackNode sn = stack[stack_ptr];
                    t = sn.dist;
                    node_index = sn.bvh_node_index;
                }
            }
        } else {
            node_index = child1;
            if (dist2 != FLT_MAX){
                stack[stack_ptr] = StackNode(child2, dist2);
                stack_ptr += 1;
            }
        }
    }
}

vec3 triangle_normal(uint triangle_id){
    vec3 p1 = triangles[triangle_id].p1;
    vec3 p2 = triangles[triangle_id].p2;
    vec3 p3 = triangles[triangle_id].p3;
    p1 = p2 - p1;
    p2 = p2 - p3;
    return normalize(my_cross(normalize(p1), normalize(p2)));
}

void main() {
    uvec3 global_invocation_id = gl_GlobalInvocationID;
    uint x = global_invocation_id.x;
    uint y = global_invocation_id.y;
    ivec2 pos = ivec2(x,y);

    vec3 dir = camera_pos
        + camera_dir
        + camera_side * (float(x) - half_width)
            / (width / ( width / height))
        + camera_up * (float(y) - half_height) /  height;

    dir = normalize(dir - camera_pos);
    vec3 dirr = 1/dir;
    float t = FLT_MAX;
    uint prim = -1;

    traverse(
        camera_pos, 
        dir, 
        dirr, 
        t, 
        prim
    );


    if (prim != uint(-1)){
        vec3 normal = triangle_normal(prim);
        vec3 color = vec3(dot(normal.xyz, normalize(vec3(1,-1,1)))+1)/2;
        imageStore(to_draw_texture, pos, vec4(color, 1));
    } else {
        imageStore(to_draw_texture, pos, vec4(0));
    }
}