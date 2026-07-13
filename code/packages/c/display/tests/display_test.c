/*
 * Tests for the C display library, using the header-only iso_test.h harness
 * (pure ISO). Expectations mirror the Rust crate's own unit tests one-for-one.
 */
#include "iso_test.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* strcmp, strlen */

#include "display.h"

/* Allocate a framebuffer for `config` and init a driver over it. Returns the
 * malloc'd memory (caller frees after use). */
static uint8_t *make_driver(DisplayDriver *d, DisplayConfig config) {
    size_t n = config.columns * config.rows * DISPLAY_BYTES_PER_CELL;
    uint8_t *mem = (uint8_t *)malloc(n);
    display_init(d, config, mem, n);
    return mem;
}

int main(void) {
    /* ── config & make_attribute ──────────────────────────────────────── */
    {
        DisplayConfig def = display_config_default();
        ISO_CHECK_EQ_UINT((unsigned)def.columns, 80u);
        ISO_CHECK_EQ_UINT((unsigned)def.rows, 25u);
        ISO_CHECK(def.framebuffer_base == 0xFFFB0000u);
        ISO_CHECK_EQ_UINT(def.default_attribute, 0x07u);
        DisplayConfig comp = display_config_compact();
        ISO_CHECK_EQ_UINT((unsigned)comp.columns, 40u);
        ISO_CHECK_EQ_UINT((unsigned)comp.rows, 10u);

        ISO_CHECK_EQ_UINT(display_make_attribute(DISPLAY_COLOR_WHITE, DISPLAY_COLOR_BLUE), 0x1Fu);
        ISO_CHECK_EQ_UINT(display_make_attribute(DISPLAY_COLOR_LIGHT_GRAY, DISPLAY_COLOR_BLACK), 0x07u);
        ISO_CHECK_EQ_UINT(display_make_attribute(DISPLAY_COLOR_WHITE, DISPLAY_COLOR_RED), 0x4Fu);
        ISO_CHECK_EQ_UINT(display_make_attribute(DISPLAY_COLOR_GREEN, DISPLAY_COLOR_BLACK), 0x02u);
    }

    /* ── constructor clears screen + cursor at origin ─────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        for (size_t r = 0; r < 10; r++)
            for (size_t c = 0; c < 40; c++) {
                DisplayCell cell = display_get_cell(&d, r, c);
                ISO_CHECK(cell.character == ' ' &&
                          cell.attribute == DISPLAY_DEFAULT_ATTRIBUTE);
            }
        DisplayCursor p = display_get_cursor(&d);
        ISO_CHECK(p.row == 0 && p.col == 0);
        free(mem);
    }

    /* ── put_char basics ──────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_put_char(&d, 'A');
        ISO_CHECK(display_get_cell(&d, 0, 0).character == 'A');
        ISO_CHECK(display_get_cell(&d, 0, 0).attribute == DISPLAY_DEFAULT_ATTRIBUTE);
        ISO_CHECK(display_get_cursor(&d).col == 1);
        display_put_char(&d, 'i');
        ISO_CHECK(display_get_cell(&d, 0, 1).character == 'i');
        ISO_CHECK(display_get_cursor(&d).col == 2);
        free(mem);
    }

    /* ── control characters ───────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_put_char(&d, 'A');
        display_put_char(&d, '\n');
        ISO_CHECK(display_get_cursor(&d).row == 1 && display_get_cursor(&d).col == 0);
        for (int i = 0; i < 5; i++) display_put_char(&d, 'x');
        display_put_char(&d, '\r');
        ISO_CHECK(display_get_cursor(&d).col == 0 && display_get_cursor(&d).row == 1);
        free(mem);

        mem = make_driver(&d, display_config_compact());
        display_put_char(&d, '\t');
        ISO_CHECK(display_get_cursor(&d).col == 8);
        display_put_char(&d, 'x'); /* col 9 */
        display_put_char(&d, '\t');
        ISO_CHECK(display_get_cursor(&d).col == 16);
        free(mem);

        mem = make_driver(&d, display_config_compact());
        display_put_char(&d, 'A');
        display_put_char(&d, 'B');
        display_put_char(&d, 0x08); /* backspace */
        ISO_CHECK(display_get_cursor(&d).col == 1);
        free(mem);

        mem = make_driver(&d, display_config_compact());
        display_put_char(&d, 0x08); /* backspace at col 0 stays */
        ISO_CHECK(display_get_cursor(&d).col == 0);
        free(mem);

        /* tab wrapping to next row (compact col 39 -> next tab stop >= 40) */
        mem = make_driver(&d, display_config_compact());
        display_set_cursor(&d, 0, 39);
        display_put_char(&d, '\t');
        ISO_CHECK(display_get_cursor(&d).row == 1 && display_get_cursor(&d).col == 0);
        free(mem);
    }

    /* ── put_char_at ──────────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_put_char_at(&d, 5, 10, 'X', 0x0F);
        ISO_CHECK(display_get_cell(&d, 5, 10).character == 'X');
        ISO_CHECK(display_get_cell(&d, 5, 10).attribute == 0x0F);
        display_set_cursor(&d, 0, 0);
        display_put_char_at(&d, 5, 10, 'Y', 0x07);
        ISO_CHECK(display_get_cursor(&d).row == 0 && display_get_cursor(&d).col == 0);
        display_put_char_at(&d, 30, 0, 'X', 0x07); /* out of bounds: no-op */
        display_put_char_at(&d, 0, 100, 'X', 0x07);
        free(mem);
    }

    /* ── puts ─────────────────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_puts(&d, "Hello");
        const char *h = "Hello";
        for (size_t i = 0; i < 5; i++)
            ISO_CHECK(display_get_cell(&d, 0, i).character == (uint8_t)h[i]);
        ISO_CHECK(display_get_cursor(&d).col == 5);
        display_puts(&d, "");
        ISO_CHECK(display_get_cursor(&d).col == 5);
        free(mem);
    }

    /* ── line wrap ────────────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        for (int i = 0; i < 40; i++) display_put_char(&d, 'A');
        ISO_CHECK(display_get_cursor(&d).row == 1 && display_get_cursor(&d).col == 0);
        display_put_char(&d, 'B');
        ISO_CHECK(display_get_cell(&d, 1, 0).character == 'B');
        free(mem);

        mem = make_driver(&d, display_config_compact());
        for (int i = 0; i < 40 * 2 + 1; i++) display_put_char(&d, 'x');
        ISO_CHECK(display_get_cursor(&d).row == 2 && display_get_cursor(&d).col == 1);
        free(mem);
    }

    /* ── scroll ───────────────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        for (size_t r = 0; r < 10; r++)
            display_put_char_at(&d, r, 0, (uint8_t)('A' + r), DISPLAY_DEFAULT_ATTRIBUTE);
        uint8_t row1 = display_get_cell(&d, 1, 0).character;
        display_set_cursor(&d, 9, 0);
        display_put_char(&d, '\n'); /* triggers scroll */
        ISO_CHECK(display_get_cell(&d, 0, 0).character == row1);
        /* last row cleared, cursor at (rows-1, 0) */
        for (size_t c = 0; c < 40; c++)
            ISO_CHECK(display_get_cell(&d, 9, c).character == ' ');
        ISO_CHECK(display_get_cursor(&d).row == 9 && display_get_cursor(&d).col == 0);
        free(mem);

        /* scroll preserves attributes */
        mem = make_driver(&d, display_config_compact());
        uint8_t attr = display_make_attribute(DISPLAY_COLOR_WHITE, DISPLAY_COLOR_BLUE);
        display_put_char_at(&d, 1, 0, 'Z', attr);
        display_set_cursor(&d, 9, 0);
        display_put_char(&d, '\n');
        ISO_CHECK(display_get_cell(&d, 0, 0).character == 'Z');
        ISO_CHECK(display_get_cell(&d, 0, 0).attribute == attr);
        free(mem);
    }

    /* ── clear ────────────────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_puts(&d, "Hello World");
        display_clear(&d);
        for (size_t r = 0; r < 10; r++)
            for (size_t c = 0; c < 40; c++)
                ISO_CHECK(display_get_cell(&d, r, c).character == ' ');
        ISO_CHECK(display_get_cursor(&d).row == 0 && display_get_cursor(&d).col == 0);
        free(mem);
    }

    /* ── snapshot ─────────────────────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_puts(&d, "Hello World");
        DisplaySnapshot s;
        ISO_CHECK(display_snapshot(&d, &s) == 1);
        ISO_CHECK(strcmp(s.lines[0], "Hello World") == 0);
        ISO_CHECK(display_snapshot_contains(&s, "Hello World"));
        ISO_CHECK(display_snapshot_contains(&s, "World"));
        ISO_CHECK(!display_snapshot_contains(&s, "Goodbye"));
        ISO_CHECK(s.rows == 10 && s.columns == 40);
        display_snapshot_free(&s);
        free(mem);

        /* trailing spaces trimmed; empty lines are "" */
        mem = make_driver(&d, display_config_compact());
        display_puts(&d, "Hi");
        ISO_CHECK(display_snapshot(&d, &s) == 1);
        ISO_CHECK(strcmp(s.lines[0], "Hi") == 0);
        for (size_t i = 1; i < s.rows; i++)
            ISO_CHECK(strcmp(s.lines[i], "") == 0);
        display_snapshot_free(&s);
        free(mem);

        /* multi-line + line_at */
        mem = make_driver(&d, display_config_compact());
        display_puts(&d, "Line 0");
        display_put_char(&d, '\n');
        display_puts(&d, "Line 1");
        ISO_CHECK(display_snapshot(&d, &s) == 1);
        ISO_CHECK(strcmp(display_snapshot_line_at(&s, 0), "Line 0") == 0);
        ISO_CHECK(strcmp(display_snapshot_line_at(&s, 1), "Line 1") == 0);
        ISO_CHECK(strcmp(display_snapshot_line_at(&s, 100), "") == 0);
        display_snapshot_free(&s);
        free(mem);

        /* to_padded: rows lines, each padded to columns width */
        mem = make_driver(&d, display_config_compact());
        display_puts(&d, "Hello");
        ISO_CHECK(display_snapshot(&d, &s) == 1);
        char *padded = display_snapshot_to_padded(&s);
        ISO_CHECK(padded != NULL);
        {
            /* split on '\n': expect `rows` lines each `columns` long */
            size_t line_count = 1, this_len = 0;
            int all_full = 1;
            for (char *p = padded; *p; p++) {
                if (*p == '\n') {
                    if (this_len != 40) all_full = 0;
                    line_count++;
                    this_len = 0;
                } else {
                    this_len++;
                }
            }
            if (this_len != 40) all_full = 0;
            ISO_CHECK(line_count == 10 && all_full);
        }
        free(padded);
        display_snapshot_free(&s);
        free(mem);

        /* snapshot records cursor */
        mem = make_driver(&d, display_config_compact());
        display_set_cursor(&d, 5, 10);
        ISO_CHECK(display_snapshot(&d, &s) == 1);
        ISO_CHECK(s.cursor.row == 5 && s.cursor.col == 10);
        display_snapshot_free(&s);
        free(mem);
    }

    /* ── cursor clamp & edge cases ────────────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        display_set_cursor(&d, 100, 100);
        ISO_CHECK(display_get_cursor(&d).row == 9 && display_get_cursor(&d).col == 39);

        DisplayCell oob = display_get_cell(&d, 100, 0);
        ISO_CHECK(oob.character == ' ' && oob.attribute == DISPLAY_DEFAULT_ATTRIBUTE);

        display_clear(&d);
        display_put_char(&d, 0x00);
        ISO_CHECK(display_get_cell(&d, 0, 0).character == 0x00);
        free(mem);
    }

    /* ── standard 80x25: put/wrap/scroll + all 256 byte values ────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_default());
        display_put_char(&d, 'A');
        ISO_CHECK(display_get_cell(&d, 0, 0).character == 'A' &&
                  display_get_cell(&d, 0, 0).attribute == 0x07);
        free(mem);

        mem = make_driver(&d, display_config_default());
        for (int i = 0; i < 81; i++) display_put_char(&d, 'A');
        ISO_CHECK(display_get_cursor(&d).row == 1 && display_get_cursor(&d).col == 1);
        free(mem);

        mem = make_driver(&d, display_config_default());
        for (size_t r = 0; r < 25; r++)
            display_put_char_at(&d, r, 0, (uint8_t)('A' + (r % 26)), DISPLAY_DEFAULT_ATTRIBUTE);
        display_set_cursor(&d, 24, 0);
        display_put_char(&d, '\n');
        ISO_CHECK(display_get_cell(&d, 0, 0).character == 'B');
        free(mem);

        mem = make_driver(&d, display_config_default());
        for (int i = 0; i < 256; i++) {
            size_t row = (size_t)i / 80, col = (size_t)i % 80;
            display_put_char_at(&d, row, col, (uint8_t)i, DISPLAY_DEFAULT_ATTRIBUTE);
        }
        for (int i = 0; i < 256; i++) {
            size_t row = (size_t)i / 80, col = (size_t)i % 80;
            ISO_CHECK(display_get_cell(&d, row, col).character == (uint8_t)i);
        }
        free(mem);
    }

    /* ── rapid scrolling stays consistent ─────────────────────────────── */
    {
        DisplayDriver d;
        uint8_t *mem = make_driver(&d, display_config_compact());
        for (int i = 0; i < 100; i++) {
            display_puts(&d, "Line");
            display_put_char(&d, '\n');
        }
        DisplaySnapshot s;
        ISO_CHECK(display_snapshot(&d, &s) == 1);
        ISO_CHECK(display_snapshot_contains(&s, "Line"));
        display_snapshot_free(&s);
        free(mem);
    }

    return ISO_TEST_RESULT();
}
