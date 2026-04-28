// GPU kernel: apply power-law gamma in linear light.
//
// Uniforms layout (1 × float32, 4 bytes):
//   [gamma]   — exponent applied per channel in linear light
//   γ < 1 brightens midtones; γ > 1 darkens; γ = 1 is identity.

#include <metal_stdlib>
using namespace metal;

struct GammaUni { float gamma; };

inline float srgb_decode(uchar c) {
    float v = float(c) / 255.0f;
    return (v <= 0.04045f) ? (v / 12.92f) : pow((v + 0.055f) / 1.055f, 2.4f);
}

inline uchar srgb_encode(float lin) {
    float c = clamp(lin, 0.0f, 1.0f);
    float s = (c <= 0.0031308f) ? (c * 12.92f) : (1.055f * pow(c, 1.0f / 2.4f) - 0.055f);
    return uchar(round(s * 255.0f));
}

kernel void gpu_gamma(
    device const uchar*    src [[buffer(0)]],
    device       uchar*    dst [[buffer(1)]],
    device const GammaUni& uni [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint o = gid * 4u;
    dst[o + 0] = srgb_encode(pow(srgb_decode(src[o + 0]), uni.gamma));
    dst[o + 1] = srgb_encode(pow(srgb_decode(src[o + 1]), uni.gamma));
    dst[o + 2] = srgb_encode(pow(srgb_decode(src[o + 2]), uni.gamma));
    dst[o + 3] = src[o + 3]; // A pass-through
}
