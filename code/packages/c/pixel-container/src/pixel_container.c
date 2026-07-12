/*
 * pixel_container.c — implementation of the RGBA8 pixel buffer (see
 * pixel_container.h). A faithful port of the Rust `pixel-container` crate's
 * `PixelContainer`, with panics replaced by NULL returns.
 */
#include "pixel_container.h"

#include <stdlib.h> /* calloc, malloc, free */
#include <string.h> /* memcpy, memcmp */

struct PixelContainer {
    uint32_t width;
    uint32_t height;
    uint8_t *data; /* width*height*4 bytes, or NULL if that is 0 */
    size_t len;
};

/* Compute width*height*4 as size_t; returns 1 with *out set, or 0 on overflow. */
static int byte_size(uint32_t width, uint32_t height, size_t *out) {
    size_t w = width, h = height, pixels;
    if (w != 0 && h > (size_t)-1 / w) {
        return 0;
    }
    pixels = w * h;
    if (pixels > (size_t)-1 / 4) {
        return 0;
    }
    *out = pixels * 4;
    return 1;
}

PixelContainer *pixel_new(uint32_t width, uint32_t height) {
    size_t size;
    PixelContainer *p;
    if (!byte_size(width, height, &size)) {
        return NULL;
    }
    p = malloc(sizeof *p);
    if (!p) {
        return NULL;
    }
    p->width = width;
    p->height = height;
    p->len = size;
    if (size == 0) {
        p->data = NULL;
    } else {
        p->data = calloc(size, 1); /* all-zero, fully transparent */
        if (!p->data) {
            free(p);
            return NULL;
        }
    }
    return p;
}

PixelContainer *pixel_from_data(uint32_t width, uint32_t height,
                                const uint8_t *data, size_t data_len) {
    size_t size;
    PixelContainer *p;
    if (!byte_size(width, height, &size)) {
        return NULL;
    }
    if (data_len != size) {
        return NULL; /* the Rust crate panics here */
    }
    p = malloc(sizeof *p);
    if (!p) {
        return NULL;
    }
    p->width = width;
    p->height = height;
    p->len = size;
    if (size == 0) {
        p->data = NULL;
    } else {
        p->data = malloc(size);
        if (!p->data) {
            free(p);
            return NULL;
        }
        memcpy(p->data, data, size);
    }
    return p;
}

PixelContainer *pixel_clone(const PixelContainer *p) {
    return pixel_from_data(p->width, p->height, p->data, p->len);
}

void pixel_free(PixelContainer *p) {
    if (p) {
        free(p->data);
        free(p);
    }
}

/* ---- accessors -------------------------------------------------------- */

uint32_t pixel_width(const PixelContainer *p) { return p->width; }
uint32_t pixel_height(const PixelContainer *p) { return p->height; }

size_t pixel_count(const PixelContainer *p) {
    return (size_t)p->width * (size_t)p->height;
}

size_t pixel_byte_count(const PixelContainer *p) { return p->len; }

const uint8_t *pixel_data(const PixelContainer *p) { return p->data; }

void pixel_at(const PixelContainer *p, uint32_t x, uint32_t y,
              uint8_t rgba[4]) {
    size_t i;
    if (x >= p->width || y >= p->height) {
        rgba[0] = rgba[1] = rgba[2] = rgba[3] = 0;
        return;
    }
    i = ((size_t)y * p->width + x) * 4;
    rgba[0] = p->data[i];
    rgba[1] = p->data[i + 1];
    rgba[2] = p->data[i + 2];
    rgba[3] = p->data[i + 3];
}

void pixel_set(PixelContainer *p, uint32_t x, uint32_t y, uint8_t r, uint8_t g,
               uint8_t b, uint8_t a) {
    size_t i;
    if (x >= p->width || y >= p->height) {
        return;
    }
    i = ((size_t)y * p->width + x) * 4;
    p->data[i] = r;
    p->data[i + 1] = g;
    p->data[i + 2] = b;
    p->data[i + 3] = a;
}

void pixel_fill(PixelContainer *p, uint8_t r, uint8_t g, uint8_t b, uint8_t a) {
    size_t i;
    for (i = 0; i + 4 <= p->len; i += 4) {
        p->data[i] = r;
        p->data[i + 1] = g;
        p->data[i + 2] = b;
        p->data[i + 3] = a;
    }
}

int pixel_equals(const PixelContainer *a, const PixelContainer *b) {
    if (a->width != b->width || a->height != b->height || a->len != b->len) {
        return 0;
    }
    if (a->len == 0) {
        return 1;
    }
    return memcmp(a->data, b->data, a->len) == 0;
}
