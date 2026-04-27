// CUDA kernel: additive brightness shift in sRGB u8, clamped to [0, 255].
//
// Arguments:
//   0: src         (CUdeviceptr → const unsigned char*)
//   1: dst         (CUdeviceptr → unsigned char*)
//   2: params      (CUdeviceptr → const int*, 1 int: delta ∈ [-255, 255])
//   3: pixel_count (unsigned int)

extern "C" __global__ void gpu_brightness(
    const unsigned char* src,
    unsigned char*       dst,
    const int*           params,
    unsigned int         pixel_count
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= pixel_count) return;

    int delta    = params[0];
    unsigned int o = gid * 4u;
    dst[o + 0] = (unsigned char)max(0, min(255, (int)src[o + 0] + delta));
    dst[o + 1] = (unsigned char)max(0, min(255, (int)src[o + 1] + delta));
    dst[o + 2] = (unsigned char)max(0, min(255, (int)src[o + 2] + delta));
    dst[o + 3] = src[o + 3]; // A pass-through
}
