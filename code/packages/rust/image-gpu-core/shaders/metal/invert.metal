// GPU kernel: invert RGB channels, preserve alpha.
//
// Thread model: one thread per pixel.
//   gid = pixel index → byte offset = gid * 4
//
// Buffer bindings (matches gpu-runtime run_pixels convention):
//   [[buffer(0)]] = src  (RGBA8, read-only)
//   [[buffer(1)]] = dst  (RGBA8, write)
//   [[buffer(2)]] = uniforms (unused for invert)

#include <metal_stdlib>
using namespace metal;

kernel void gpu_invert(
    device const uchar* src [[buffer(0)]],
    device       uchar* dst [[buffer(1)]],
    device const uchar* uni [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint o = gid * 4u;
    dst[o + 0] = 255u - src[o + 0]; // R
    dst[o + 1] = 255u - src[o + 1]; // G
    dst[o + 2] = 255u - src[o + 2]; // B
    dst[o + 3] = src[o + 3];         // A pass-through
}
