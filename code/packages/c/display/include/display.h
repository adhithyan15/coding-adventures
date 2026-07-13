/*
 * display.h — VGA text-mode framebuffer simulation, pure ISO C17.
 * ==============================================================
 *
 * A faithful port of the Rust `display` crate. Simulates the classic 80x25 VGA
 * text-mode framebuffer: a grid of cells, each 2 bytes (byte 0 = character,
 * byte 1 = colour attribute). The driver tracks a cursor, interprets a few
 * control characters (\n \r \t backspace), wraps at the right edge, and scrolls
 * when output runs past the bottom.
 *
 * ## Memory ownership
 *
 * The framebuffer memory is **caller-owned**: you supply a buffer of at least
 * `columns * rows * DISPLAY_BYTES_PER_CELL` bytes (mirroring the Rust crate's
 * borrowed `&mut [u8]`). The `DisplayDriver` only borrows it; nothing here
 * allocates the framebuffer. `display_snapshot` DOES allocate a text view that
 * you release with `display_snapshot_free`.
 *
 * ## Divergences from Rust (documented)
 *
 *   - Rust `Vec<String>` snapshot lines -> a malloc'd `char **` of
 *     NUL-terminated trimmed lines (embedded NUL bytes, which the Rust `String`
 *     could hold, terminate a C line early — untested, and framebuffers only
 *     ever hold printable text in practice).
 *   - Out-of-range framebuffer access is a defensive no-op here rather than the
 *     Rust bounds-check panic; correctly sized buffers never hit it.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no <math.h>, no compiler extensions.
 */
#ifndef CA_DISPLAY_H
#define CA_DISPLAY_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Constants ────────────────────────────────────────────────────────────*/

#define DISPLAY_BYTES_PER_CELL 2
#define DISPLAY_DEFAULT_COLUMNS 80
#define DISPLAY_DEFAULT_ROWS 25
#define DISPLAY_DEFAULT_FRAMEBUFFER_BASE 0xFFFB0000u
#define DISPLAY_DEFAULT_ATTRIBUTE 0x07u /* light gray on black */

/* VGA 16-colour palette. */
typedef enum {
    DISPLAY_COLOR_BLACK = 0,
    DISPLAY_COLOR_BLUE = 1,
    DISPLAY_COLOR_GREEN = 2,
    DISPLAY_COLOR_CYAN = 3,
    DISPLAY_COLOR_RED = 4,
    DISPLAY_COLOR_MAGENTA = 5,
    DISPLAY_COLOR_BROWN = 6,
    DISPLAY_COLOR_LIGHT_GRAY = 7,
    DISPLAY_COLOR_DARK_GRAY = 8,
    DISPLAY_COLOR_LIGHT_BLUE = 9,
    DISPLAY_COLOR_LIGHT_GREEN = 10,
    DISPLAY_COLOR_LIGHT_CYAN = 11,
    DISPLAY_COLOR_LIGHT_RED = 12,
    DISPLAY_COLOR_LIGHT_MAGENTA = 13,
    DISPLAY_COLOR_YELLOW = 14,
    DISPLAY_COLOR_WHITE = 15
} DisplayColor;

/* Combine a foreground and background colour into an attribute byte
 * (fg in the low 4 bits, bg in bits 4-6). */
uint8_t display_make_attribute(uint8_t fg, uint8_t bg);

/* ── Data structures ──────────────────────────────────────────────────────*/

typedef struct {
    uint8_t character;
    uint8_t attribute;
} DisplayCell;

typedef struct {
    size_t row;
    size_t col;
} DisplayCursor;

typedef struct {
    size_t columns;
    size_t rows;
    uint32_t framebuffer_base;
    uint8_t default_attribute;
} DisplayConfig;

DisplayConfig display_config_default(void); /* 80x25 */
DisplayConfig display_config_compact(void); /* 40x10, for tests */

/* The driver borrows `memory` (length `memory_len`); it does not own it. */
typedef struct {
    DisplayConfig config;
    uint8_t *memory;
    size_t memory_len;
    DisplayCursor cursor;
} DisplayDriver;

/* ── Driver lifecycle ─────────────────────────────────────────────────────*/

/* Initialise a driver over `memory` and clear the screen (cursor -> 0,0). */
void display_init(DisplayDriver *d, DisplayConfig config, uint8_t *memory,
                  size_t memory_len);

/* Wrap an existing framebuffer WITHOUT clearing it (cursor -> 0,0). */
void display_wrap(DisplayDriver *d, DisplayConfig config, uint8_t *memory,
                  size_t memory_len);

/* ── Writing ──────────────────────────────────────────────────────────────*/

/* Write one character at the cursor (default attribute) and advance. Handles
 * \n (0x0A), \r (0x0D), \t (0x09), backspace (0x08); wraps and scrolls. */
void display_put_char(DisplayDriver *d, uint8_t ch);

/* Write `ch`+`attr` at (row,col) without moving the cursor or interpreting
 * control characters. Out-of-range positions are ignored. */
void display_put_char_at(DisplayDriver *d, size_t row, size_t col, uint8_t ch,
                         uint8_t attr);

/* Write a NUL-terminated string one byte at a time (via display_put_char). */
void display_puts(DisplayDriver *d, const char *s);

/* ── Screen / cursor management ───────────────────────────────────────────*/

void display_clear(DisplayDriver *d);   /* fill with spaces, cursor -> 0,0 */
void display_scroll(DisplayDriver *d);  /* shift up one row, clear last row */
void display_set_cursor(DisplayDriver *d, size_t row, size_t col); /* clamped */
DisplayCursor display_get_cursor(const DisplayDriver *d);

/* Character + attribute at (row,col); {' ', default_attribute} if out of range. */
DisplayCell display_get_cell(const DisplayDriver *d, size_t row, size_t col);

/* ── Snapshot ─────────────────────────────────────────────────────────────*/

/* A frozen text view of the display: one trimmed, NUL-terminated line per row. */
typedef struct {
    char **lines; /* `rows` entries, trailing whitespace trimmed */
    size_t rows;
    size_t columns;
    DisplayCursor cursor;
} DisplaySnapshot;

/* Build a snapshot (allocates). Returns 1 on success, 0 on allocation failure
 * (leaving *snap zeroed). Release with display_snapshot_free. */
int display_snapshot(const DisplayDriver *d, DisplaySnapshot *snap);

void display_snapshot_free(DisplaySnapshot *snap);

/* True if `text` appears in any (trimmed) line. */
int display_snapshot_contains(const DisplaySnapshot *snap, const char *text);

/* The trimmed text of row `row`, or "" if out of range. Points into `snap`. */
const char *display_snapshot_line_at(const DisplaySnapshot *snap, size_t row);

/* Render every row padded to the full column width, joined by '\n'. Returns a
 * malloc'd NUL-terminated string (caller frees) or NULL on allocation failure. */
char *display_snapshot_to_padded(const DisplaySnapshot *snap);

#ifdef __cplusplus
}
#endif

#endif /* CA_DISPLAY_H */
