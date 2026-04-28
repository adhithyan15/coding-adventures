// GPU kernel: additive brightness shift in sRGB u8.
//
// Uniforms layout (1 × int32, 4 bytes):
//   [delta]   — signed additive offset ∈ [-255, 255], clamped to [0, 255]
// Alpha is unchanged.

#include <metal_stdlib>
using namespace metal;

struct BrightnessUni { int delta; };

kernel void gpu_brightness(
    device const uchar*         src [[buffer(0)]],
    device       uchar*         dst [[buffer(1)]],
    device const BrightnessUni& uni [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint o = gid * 4u;
    dst[o + 0] = uchar(clamp(int(src[o + 0]) + uni.delta, 0, 255));
    dst[o + 1] = uchar(clamp(int(src[o + 1]) + uni.delta, 0, 255));
    dst[o + 2] = uchar(clamp(int(src[o + 2]) + uni.delta, 0, 255));
    dst[o + 3] = src[o + 3]; // A pass-through
}
