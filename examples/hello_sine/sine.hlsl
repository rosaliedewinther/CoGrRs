RWTexture2D<float4> to_draw_texture;

struct GpuData
{
    float time;
    uint width;
    uint height;
};

[numthreads(32, 32, 1)] void main(uint2 threadId
                                  : SV_DispatchThreadID)
{
    float val = sin(float(threadId.x * 5) / 1920 + 1.0) / 2 + 0.5;           // calulate sin value at certain x
    bool color = val * 1080 < threadId.y + 1 && val * 1080 > threadId.y - 1; // the pixel has to be colored if it is at most 1 pixel away from the sin value

    to_draw_texture[threadId] = float4(color, 0, 0, 1);
}
