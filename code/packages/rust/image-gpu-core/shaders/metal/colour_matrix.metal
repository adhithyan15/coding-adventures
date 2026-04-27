// GPU kernel: apply a 3×3 colour matrix in linear light.
//
// Uniforms layout (9 × float32, little-endian, 36 bytes):
//   [m00, m01, m02, m10, m11, m12, m20, m21, m22]
//   out_rgb = M × in_rgb  (each channel decoded sRGB → linear, then re-encoded)

#include <metal_stdlib>
using namespace metal;

struct ColourMatrixUni {
    float m00, m01, m02;
    float m10, m11, m12;
    float m20, m21, m22;
};

// sRGB u8 → linear float [0, 1]
inline float srgb_decode(uchar c) {
    float v = float(c) / 255.0f;
    return (v <= 0.04045f) ? (v / 12.92f) : pow((v + 0.055f) / 1.055f, 2.4f);
}

// linear float [0, 1] → sRGB u8
inline uchar srgb_encode(float lin) {
    float c = clamp(lin, 0.0f, 1.0f);
    float s = (c <= 0.0031308f) ? (c * 12.92f) : (1.055f * pow(c, 1.0f / 2.4f) - 0.055f);
    return uchar(round(s * 255.0f));
}

kernel void gpu_colour_matrix(
    device const uchar*          src [[buffer(0)]],
    device       uchar*          dst [[buffer(1)]],
    device const ColourMatrixUni& uni [[buffer(2)]],
    uint gid [[thread_position_in_grid]]
) {
    uint o  = gid * 4u;
    float rl = srgb_decode(src[o + 0]);
    float gl = srgb_decode(src[o + 1]);
    float bl = srgb_decode(src[o + 2]);

    dst[o + 0] = srgb_encode(uni.m00 * rl + uni.m01 * gl + uni.m02 * bl);
    dst[o + 1] = srgb_encode(uni.m10 * rl + uni.m11 * gl + uni.m12 * bl);
    dst[o + 2] = srgb_encode(uni.m20 * rl + uni.m21 * gl + uni.m22 * bl);
    dst[o + 3] = src[o + 3]; // A pass-through
}
