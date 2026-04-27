// CUDA kernel: convert RGBA8 to greyscale in linear light.
//
// Arguments:
//   0: src         (CUdeviceptr → const unsigned char*)
//   1: dst         (CUdeviceptr → unsigned char*)
//   2: weights     (CUdeviceptr → const float*, 3 floats: wr, wg, wb)
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

extern "C" __global__ void gpu_greyscale(
    const unsigned char* src,
    unsigned char*       dst,
    const float*         weights,
    unsigned int         pixel_count
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= pixel_count) return;

    unsigned int o  = gid * 4u;
    float y = weights[0] * srgb_decode(src[o + 0])
            + weights[1] * srgb_decode(src[o + 1])
            + weights[2] * srgb_decode(src[o + 2]);
    unsigned char v = srgb_encode(y);
    dst[o + 0] = v;
    dst[o + 1] = v;
    dst[o + 2] = v;
    dst[o + 3] = src[o + 3]; // A pass-through
}
