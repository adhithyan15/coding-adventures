// CUDA kernel: apply power-law gamma in linear light.
//
// Arguments:
//   0: src         (CUdeviceptr → const unsigned char*)
//   1: dst         (CUdeviceptr → unsigned char*)
//   2: params      (CUdeviceptr → const float*, 1 float: gamma exponent)
//   3: pixel_count (unsigned int)

__device__ __forceinline__ float srgb_decode(unsigned char c) {
    float v = c / 255.0f;
    return (v <= 0.04045f) ? (v / 12.92f) : powf((v + 0.055f) / 1.055f, 2.4f);
}

__device__ __forceinline__ unsigned char srgb_encode(float lin) {
    float c = fminf(fmaxf(lin, 0.0f), 1.0f);
    float s = (c <= 0.0031308f) ? (c * 12.92f) : (1.055f * powf(c, 1.0f / 2.4f) - 0.055f);
    return (unsigned char)(roundf(s * 255.0f));
}

extern "C" __global__ void gpu_gamma(
    const unsigned char* src,
    unsigned char*       dst,
    const float*         params,
    unsigned int         pixel_count
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= pixel_count) return;

    float g = params[0];
    unsigned int o = gid * 4u;
    dst[o + 0] = srgb_encode(powf(srgb_decode(src[o + 0]), g));
    dst[o + 1] = srgb_encode(powf(srgb_decode(src[o + 1]), g));
    dst[o + 2] = srgb_encode(powf(srgb_decode(src[o + 2]), g));
    dst[o + 3] = src[o + 3]; // A pass-through
}
