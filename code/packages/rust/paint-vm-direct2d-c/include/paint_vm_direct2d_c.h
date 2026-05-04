#ifndef CODING_ADVENTURES_PAINT_VM_DIRECT2D_C_H
#define CODING_ADVENTURES_PAINT_VM_DIRECT2D_C_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct paint_rgba8_color_t {
    uint8_t r;
    uint8_t g;
    uint8_t b;
    uint8_t a;
} paint_rgba8_color_t;

typedef struct paint_rect_instruction_t {
    uint32_t x;
    uint32_t y;
    uint32_t width;
    uint32_t height;
    paint_rgba8_color_t fill;
} paint_rect_instruction_t;

enum {
    PAINT_TEXT_ALIGN_LEFT = 0,
    PAINT_TEXT_ALIGN_CENTER = 1,
    PAINT_TEXT_ALIGN_RIGHT = 2
};

typedef struct paint_text_instruction_t {
    double x;
    double y;
    const uint8_t *text;
    size_t text_len;
    const uint8_t *font_ref;
    size_t font_ref_len;
    double font_size;
    paint_rgba8_color_t fill;
    uint32_t text_align;
} paint_text_instruction_t;

typedef struct paint_rgba8_buffer_t {
    uint32_t width;
    uint32_t height;
    uint8_t *data;
    size_t len;
} paint_rgba8_buffer_t;

uint8_t paint_vm_direct2d_render_rect_scene(
    uint32_t width,
    uint32_t height,
    paint_rgba8_color_t background,
    const paint_rect_instruction_t *rects,
    size_t rect_count,
    paint_rgba8_buffer_t *out_buffer
);

uint8_t paint_vm_direct2d_render_scene(
    uint32_t width,
    uint32_t height,
    paint_rgba8_color_t background,
    const paint_rect_instruction_t *rects,
    size_t rect_count,
    const paint_text_instruction_t *texts,
    size_t text_count,
    paint_rgba8_buffer_t *out_buffer
);

uint8_t paint_vm_gdi_render_scene(
    uint32_t width,
    uint32_t height,
    paint_rgba8_color_t background,
    const paint_rect_instruction_t *rects,
    size_t rect_count,
    const paint_text_instruction_t *texts,
    size_t text_count,
    paint_rgba8_buffer_t *out_buffer
);

void paint_vm_direct2d_free_buffer_data(uint8_t *data, size_t len);

#ifdef __cplusplus
}
#endif

#endif
