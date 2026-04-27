// CUDA kernel: apply a 3×3 colour matrix in linear light.
//
// Uniforms: 9 float32 values in row-major order [m00,m01,m02, m10,m11,m12, m20,m21,m22]
// out_rgb = M × in_rgb  (per channel: sRGB decode → multiply → sRGB encode)
//
// Arguments:
//   0: src   (CUdeviceptr → const unsigned char*)
//   1: dst   (CUdeviceptr → unsigned char*)
//   2: mat   (CUdeviceptr → const float*, 9 floats = 36 bytes)
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

extern "C" __global__ void gpu_colour_matrix(
    const unsigned char* src,
    unsigned char*       dst,
    const float*         mat,
    unsigned int         pixel_count
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= pixel_count) return;

    unsigned int o  = gid * 4u;
    float rl = srgb_decode(src[o + 0]);
    float gl = srgb_decode(src[o + 1]);
    float bl = srgb_decode(src[o + 2]);

    dst[o + 0] = srgb_encode(mat[0]*rl + mat[1]*gl + mat[2]*bl);
    dst[o + 1] = srgb_encode(mat[3]*rl + mat[4]*gl + mat[5]*bl);
    dst[o + 2] = srgb_encode(mat[6]*rl + mat[7]*gl + mat[8]*bl);
    dst[o + 3] = src[o + 3]; // A pass-through
}
