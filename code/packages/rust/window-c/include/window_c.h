#ifndef CODING_ADVENTURES_WINDOW_C_H
#define CODING_ADVENTURES_WINDOW_C_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct window_c_logical_size_t {
    double width;
    double height;
} window_c_logical_size_t;

typedef struct window_c_physical_size_t {
    uint32_t width;
    uint32_t height;
} window_c_physical_size_t;

typedef enum window_c_surface_preference_t {
    WINDOW_C_SURFACE_DEFAULT = 0,
    WINDOW_C_SURFACE_METAL = 1,
    WINDOW_C_SURFACE_DIRECT2D = 2,
    WINDOW_C_SURFACE_CAIRO = 3,
    WINDOW_C_SURFACE_CANVAS2D = 4
} window_c_surface_preference_t;

typedef enum window_c_mount_target_kind_t {
    WINDOW_C_MOUNT_NATIVE = 0,
    WINDOW_C_MOUNT_BROWSER_BODY = 1,
    WINDOW_C_MOUNT_ELEMENT_ID = 2,
    WINDOW_C_MOUNT_QUERY_SELECTOR = 3
} window_c_mount_target_kind_t;

typedef struct window_c_mount_target_t {
    uint32_t kind;
    const char *value;
} window_c_mount_target_t;

typedef struct window_c_window_attributes_t {
    const char *title;
    window_c_logical_size_t initial_size;
    uint8_t has_min_size;
    window_c_logical_size_t min_size;
    uint8_t has_max_size;
    window_c_logical_size_t max_size;
    uint8_t visible;
    uint8_t resizable;
    uint8_t decorations;
    uint8_t transparent;
    uint32_t preferred_surface;
    window_c_mount_target_t mount_target;
} window_c_window_attributes_t;

typedef enum window_c_render_target_kind_t {
    WINDOW_C_RENDER_TARGET_NONE = 0,
    WINDOW_C_RENDER_TARGET_APPKIT = 1,
    WINDOW_C_RENDER_TARGET_WIN32 = 2,
    WINDOW_C_RENDER_TARGET_BROWSER_CANVAS = 3,
    WINDOW_C_RENDER_TARGET_WAYLAND = 4,
    WINDOW_C_RENDER_TARGET_X11 = 5
} window_c_render_target_kind_t;

typedef struct window_c_appkit_render_target_t {
    uintptr_t ns_window;
    uintptr_t ns_view;
    uintptr_t metal_layer;
    uint8_t has_metal_layer;
} window_c_appkit_render_target_t;

typedef struct window_c_window_t window_c_window_t;

const char *window_c_last_error_message(void);
uint8_t window_c_is_appkit_available(void);
window_c_window_t *window_c_create_window(const window_c_window_attributes_t *attributes);
void window_c_window_free(window_c_window_t *window);
uint64_t window_c_window_id(const window_c_window_t *window);
double window_c_window_scale_factor(const window_c_window_t *window);
uint8_t window_c_window_logical_size(const window_c_window_t *window, window_c_logical_size_t *out_size);
uint8_t window_c_window_physical_size(const window_c_window_t *window, window_c_physical_size_t *out_size);
uint8_t window_c_window_request_redraw(const window_c_window_t *window);
uint8_t window_c_window_set_title(const window_c_window_t *window, const char *title);
uint8_t window_c_window_set_visible(const window_c_window_t *window, uint8_t visible);
window_c_render_target_kind_t window_c_window_render_target_kind(const window_c_window_t *window);
uint8_t window_c_window_render_target_appkit(
    const window_c_window_t *window,
    window_c_appkit_render_target_t *out_target
);

#ifdef __cplusplus
}
#endif

#endif
