struct BVHNode
{
    float minx;
    float miny;
    float minz;
    float maxx;
    float maxy;
    float maxz;
    int left_first;
    int count;
};

struct StackNode
{
    uint bvh_node_index;
    float dist;
};

struct Triangle
{
    float4 p1;
    float4 p2;
    float4 p3;
    float4 p4;
};

struct GpuData
{
    float4 camera_dir;
    float4 camera_pos;
    float4 camera_side;
    float4 camera_up;
    float width;
    float half_width;
    float height;
    float half_height;
    float time;
    uint padding1;
    uint padding2;
    uint padding3;
};

RWTexture2D<unorm float4> to_draw_texture;
StructuredBuffer<Triangle> triangles;
StructuredBuffer<BVHNode> bvh_nodes;
[[vk::push_constant]] GpuData gpu_data;

#define FLT_MAX 3.402823466e+38

float4 my_cross(float4 a, float4 b)
{
    return float4(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
        0);
}

void intersect_ray_triangle(
    in float4 ray_o,
    in float4 ray_dir,
    inout float ray_t,
    inout uint prim_id,
    in uint triangle_id)
{
    float4 p1 = triangles[triangle_id].p1;
    float4 p2 = triangles[triangle_id].p2;
    float4 p3 = triangles[triangle_id].p3;
    float4 p1_to_p2 = p2 - p1;
    float4 p1_to_p3 = p3 - p1;
    float4 uvec = my_cross(ray_dir, p1_to_p3);
    float det = dot(p1_to_p2, uvec);
    float inv_det = 1 / det;
    float4 a_to_origin = ray_o - p1;
    float u = dot(a_to_origin, uvec) * inv_det;
    if (u < 0 || u > 1)
    {
        return;
    }
    float4 vvec = my_cross(a_to_origin, p1_to_p2);
    float v = dot(ray_dir, vvec) * inv_det;
    if (v < 0 || u + v > 1)
    {
        return;
    }
    float dist = dot(p1_to_p3, vvec) * inv_det;
    if (dist > 0.00000001 && dist < ray_t)
    {
        ray_t = dist;
        prim_id = triangle_id;
    }
}

float intersect_ray_aabb(
    in float4 ray_o,
    in float4 ray_dir,
    in float4 ray_dirr,
    inout float ray_t,
    inout uint prim_id,
    in uint aabb_id)
{
    BVHNode node = bvh_nodes[aabb_id];
    float4 v_max = float4(node.maxx, node.maxy, node.maxz, 0);
    float4 v_min = float4(node.minx, node.miny, node.minz, 0);
    float4 tMin = (v_min - ray_o) * ray_dirr;
    float4 tMax = (v_max - ray_o) * ray_dirr;
    float4 t1 = min(tMin, tMax);
    float4 t2 = max(tMin, tMax);
    float tNear = max(max(t1.x, t1.y), t1.z);
    float tFar = min(min(t2.x, t2.y), t2.z);
    if (tFar >= tNear && tNear < ray_t && tFar > 0)
    {
        return tNear;
    }
    else
    {
        return FLT_MAX;
    }
}

void traverse(
    in float4 ray_o,
    in float4 ray_dir,
    in float4 ray_dirr,
    inout float ray_t,
    inout uint prim_id)
{
    StackNode stack[32];
    uint node_index = 0;
    uint stack_ptr = 0;
    while (true)
    {
        if (bvh_nodes[node_index].count > 0)
        {
            for (int i = 0; i < bvh_nodes[node_index].count; i++)
            {
                intersect_ray_triangle(
                    ray_o,
                    ray_dir,
                    ray_t,
                    prim_id,
                    bvh_nodes[node_index].left_first + i);
            }
            if (stack_ptr == 0)
            {
                break;
            }
            else
            {
                float t = FLT_MAX;
                while (t >= ray_t)
                {
                    if (stack_ptr == 0)
                    {
                        return;
                    }
                    stack_ptr -= 1;
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
            child1);
        float dist2 = intersect_ray_aabb(
            ray_o,
            ray_dir,
            ray_dirr,
            ray_t,
            prim_id,
            child2);
        if (dist1 > dist2)
        {
            uint tempu = child1;
            child1 = child2;
            child2 = tempu;
            float tempf = dist1;
            dist1 = dist2;
            dist2 = tempf;
        }
        if (dist1 == FLT_MAX)
        {
            if (stack_ptr == 0)
            {
                return;
            }
            else
            {
                float t = FLT_MAX;
                while (t >= ray_t)
                {
                    if (stack_ptr == 0)
                    {
                        return;
                    }
                    stack_ptr -= 1;
                    StackNode sn = stack[stack_ptr];
                    t = sn.dist;
                    node_index = sn.bvh_node_index;
                }
            }
        }
        else
        {
            node_index = child1;
            if (dist2 != FLT_MAX)
            {
                stack[stack_ptr].bvh_node_index = child2;
                stack[stack_ptr].dist = dist2;
                stack_ptr += 1;
            }
        }
    }
}

float4 triangle_normal(uint triangle_id)
{
    float4 p1 = triangles[triangle_id].p1;
    float4 p2 = triangles[triangle_id].p2;
    float4 p3 = triangles[triangle_id].p3;
    p1 = p2 - p1;
    p2 = p2 - p3;
    return normalize(my_cross(normalize(p1), normalize(p2)));
}

[numthreads(32, 32, 1)] void main(uint2 threadId
                                  : SV_DispatchThreadID)
{
    uint x = threadId.x;
    uint y = threadId.y;
    int2 pos = int2(x, y);

    float4 dir = gpu_data.camera_pos + gpu_data.camera_dir + gpu_data.camera_side * (float(x) - gpu_data.half_width) / (gpu_data.width / (gpu_data.width / gpu_data.height)) + gpu_data.camera_up * (float(y) - gpu_data.half_height) / gpu_data.height;

    dir = normalize(dir - gpu_data.camera_pos);
    float4 dirr = 1 / dir;
    float t = FLT_MAX;
    uint prim = -1;

    traverse(
        gpu_data.camera_pos,
        dir,
        dirr,
        t,
        prim);

    if (prim != uint(-1))
    {
        float4 normal = triangle_normal(prim);
        float intensity = dot(normal.xyz, normalize(float3(1, -1, 1))) + 1;
        float4 color = float4(intensity, intensity, intensity, intensity) / 2;
        to_draw_texture[pos] = color;
    }
    else
    {
        to_draw_texture[pos] = float4(0, 0, 0, 0);
    }
}
