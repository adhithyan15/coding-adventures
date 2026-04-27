// GPU kernel: convert RGBA8 to greyscale in linear light.
//
// Uniforms layout (3 × float32, 12 bytes):
//   [wr, wg, wb]   — luminance weights (must sum to 1.0)
//   Y = wr·R + wg·G + wb·B  (computed in linear light, re-encoded to sRGB)

#include <metal_stdlib>
using namespace metal;

struct GreyscaleUni { float wr, wg, wb; };

inline float srgb_decode(uchar c) {
    float v = float(c) / 255.0f;
    return (v <= 0.04045f) ? (v / 12.92f) : pow((v + 0.055f) / 1.055f, 2.4f);
}

inline uchar srgb_encode(float lin) {
    float c = clamp(lin, 0.0f, 1.0f);
    float s = (c <= 0.0031308f) ? (c * 12.92f) : (1.055f * pow(c, 1.0f / 2.4f) - 0.055f);
    return uchar(round(s * 255.0f));
}

kernel void gpu_greyscale(
    device const uchar*       src [[buffer(0)]],
    device       uchar*       dst [[buffer(1)]],
    device const GreyscaleUni& uni [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint o = gid * 4u;
    float y = uni.wr * srgb_decode(src[o + 0])
            + uni.wg * srgb_decode(src[o + 1])
            + uni.wb * srgb_decode(src[o + 2]);
    uchar v    = srgb_encode(y);
    dst[o + 0] = v;
    dst[o + 1] = v;
    dst[o + 2] = v;
    dst[o + 3] = src[o + 3]; // A pass-through
}
