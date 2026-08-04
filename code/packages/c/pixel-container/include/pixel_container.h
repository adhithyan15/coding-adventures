/*
 * pixel_container.h — a flat, row-major RGBA8 pixel buffer, in pure ISO C17.
 * A faithful port of the Rust `pixel-container` crate's `PixelContainer` type.
 * ===========================================================================
 *
 * PixelContainer is the universal interchange type between renderers and image
 * codecs: 4 bytes per pixel in RGBA order, row-major from the top-left.
 *
 *   offset = (y * width + x) * 4
 *   data[offset + 0] = R,  +1 = G,  +2 = B,  +3 = A
 *
 * A fully opaque pixel has A = 255; a fully transparent one has A = 0.
 *
 * OWNERSHIP. Constructors return a malloc'd handle the caller frees with
 * `pixel_free`. Where the Rust crate panics (dimension overflow, a from_data
 * length mismatch), this port returns NULL instead — a library should not abort
 * the process.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef PIXEL_CONTAINER_H
#define PIXEL_CONTAINER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t */

typedef struct PixelContainer PixelContainer;

/* pixel_new — a blank (all-zero, fully transparent) buffer of `width`×`height`.
 * Returns NULL if width*height*4 overflows size_t or on allocation failure. */
PixelContainer *pixel_new(uint32_t width, uint32_t height);

/* pixel_from_data — a buffer initialised from `data` (which must have exactly
 * width*height*4 bytes). Returns NULL on a length mismatch, overflow, or OOM. */
PixelContainer *pixel_from_data(uint32_t width, uint32_t height,
                                const uint8_t *data, size_t data_len);

/* pixel_clone — an independent deep copy. NULL on allocation failure. */
PixelContainer *pixel_clone(const PixelContainer *p);

/* pixel_free — release a container (safe with NULL). */
void pixel_free(PixelContainer *p);

/* ---- accessors -------------------------------------------------------- */

uint32_t pixel_width(const PixelContainer *p);
uint32_t pixel_height(const PixelContainer *p);
size_t pixel_count(const PixelContainer *p);      /* width * height */
size_t pixel_byte_count(const PixelContainer *p); /* width * height * 4 */

/* pixel_data — the raw RGBA8 buffer (pixel_byte_count bytes), or NULL if empty. */
const uint8_t *pixel_data(const PixelContainer *p);

/* pixel_at — read the RGBA components at (x, y) into `rgba` (4 bytes). Writes
 * {0,0,0,0} if the coordinates are out of bounds. */
void pixel_at(const PixelContainer *p, uint32_t x, uint32_t y,
              uint8_t rgba[4]);

/* pixel_set — write the RGBA components at (x, y). No-op if out of bounds. */
void pixel_set(PixelContainer *p, uint32_t x, uint32_t y, uint8_t r, uint8_t g,
               uint8_t b, uint8_t a);

/* pixel_fill — fill the whole buffer with one RGBA colour. */
void pixel_fill(PixelContainer *p, uint8_t r, uint8_t g, uint8_t b, uint8_t a);

/* pixel_equals — 1 iff width, height, and every byte match. */
int pixel_equals(const PixelContainer *a, const PixelContainer *b);

#endif /* PIXEL_CONTAINER_H */
