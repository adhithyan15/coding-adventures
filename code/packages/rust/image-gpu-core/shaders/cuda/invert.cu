// CUDA kernel: invert RGB channels, preserve alpha.
//
// Thread model: one thread per pixel.
//   gid = blockIdx.x * blockDim.x + threadIdx.x  → pixel index
//   byte offset = gid * 4
//
// Arguments (passed via cuLaunchKernel params array):
//   0: src        (CUdeviceptr → const unsigned char*)
//   1: dst        (CUdeviceptr → unsigned char*)
//   2: pixel_count (unsigned int)

extern "C" __global__ void gpu_invert(
    const unsigned char* src,
    unsigned char*       dst,
    unsigned int         pixel_count
) {
    unsigned int gid = blockIdx.x * blockDim.x + threadIdx.x;
    if (gid >= pixel_count) return;

    unsigned int o = gid * 4u;
    dst[o + 0] = 255u - src[o + 0]; // R
    dst[o + 1] = 255u - src[o + 1]; // G
    dst[o + 2] = 255u - src[o + 2]; // B
    dst[o + 3] = src[o + 3];         // A pass-through
}
