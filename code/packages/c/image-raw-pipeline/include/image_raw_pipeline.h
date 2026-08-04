/*
 * image_raw_pipeline.h — shared RAW colour-development pipeline, pure ISO C17.
 * =========================================================================
 *
 * A faithful port of the Rust `image-raw-pipeline` crate. Camera RAW formats
 * (TIFF, DNG, CR2, NEF, ARW, RAF, ORF, RW2) all need the same four-stage
 * colour pipeline to turn raw sensor values into a displayable sRGB image:
 *
 *     normalize (black level) -> white balance -> colour matrix -> sRGB gamma
 *
 * ## API
 *
 *   - srgb_gamma / srgb_decode : the IEC 61966-2-1 sRGB transfer functions
 *     (piecewise linear + power law) and their inverse.
 *   - mat3x3_mul               : 3x3 matrix times a column vector (hot path).
 *   - invert_3x3               : analytic 3x3 inversion via Cramer's rule.
 *   - apply_color_pipeline     : the full four-stage development.
 *
 * ## No libm
 *
 * `srgb_gamma`/`srgb_decode` need a fractional `pow`; it is computed from
 * scratch (see the .c file), so the package needs no `<math.h>` and no `-lm`.
 *
 * ## Divergences from Rust (documented)
 *
 *   - Rust `Option<[[f64;3];3]>` (invert) -> a `1/0` success flag plus an
 *     out-parameter.
 *   - Rust `Vec<(u8,u8,u8)>` (pipeline) -> a malloc'd `IrpRgb8 *` the caller
 *     frees; an `IrpStatus` reports OK / overflow / OOM.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef CA_IMAGE_RAW_PIPELINE_H
#define CA_IMAGE_RAW_PIPELINE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* One raw sensor pixel: three 16-bit channels (post-demosaic). */
typedef struct {
    uint16_t r;
    uint16_t g;
    uint16_t b;
} IrpRaw16;

/* One developed sRGB pixel: three 8-bit channels. */
typedef struct {
    uint8_t r;
    uint8_t g;
    uint8_t b;
} IrpRgb8;

typedef enum {
    IRP_OK = 0,   /* success */
    IRP_ERR_ALLOC /* out of memory, or a size_t multiply overflowed */
} IrpStatus;

/* ── sRGB transfer functions ──────────────────────────────────────────────*/

/* sRGB EOTF: linear light -> display encoding. Fixed points gamma(0)=0,
 * gamma(1)=1. Inputs outside [0,1] are NOT clamped (the caller decides). */
double irp_srgb_gamma(double linear);

/* Inverse sRGB EOTF: display encoding -> linear light. */
double irp_srgb_decode(double encoded);

/* ── 3x3 matrix ops ───────────────────────────────────────────────────────*/

/* out = m * v, with m row-major (out[i] = sum_j m[i][j]*v[j]). `out` must not
 * alias `v`. */
void irp_mat3x3_mul(const double m[3][3], const double v[3], double out[3]);

/* Analytic 3x3 inverse via Cramer's rule. Returns 1 and writes `out` on
 * success; returns 0 (leaving `out` untouched) when |det| < 1e-12. */
int irp_invert_3x3(const double m[3][3], double out[3][3]);

/* ── Full pipeline ────────────────────────────────────────────────────────*/

/* Develop `n` raw pixels into sRGB. On IRP_OK, *out points to a malloc'd array
 * of `n` IrpRgb8 (NULL when n == 0) that the caller must free(). On error, *out
 * is set to NULL. `wb` are the [R,G,B] white-balance multipliers;
 * `color_matrix` is the row-major camera->sRGB matrix. */
IrpStatus irp_apply_color_pipeline(const IrpRaw16 *pixels, size_t n,
                                   uint32_t black_level, uint32_t white_level,
                                   const double wb[3],
                                   const double color_matrix[3][3],
                                   IrpRgb8 **out);

#ifdef __cplusplus
}
#endif

#endif /* CA_IMAGE_RAW_PIPELINE_H */
