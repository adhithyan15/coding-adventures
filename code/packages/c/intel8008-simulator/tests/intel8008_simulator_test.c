/*
 * Tests for intel8008-simulator, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's unit tests.
 */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "intel8008_simulator.h"

/* Run a fixed program and return the simulator (caller frees). */
static void run(I8008Sim *s, const uint8_t *prog, size_t len) {
    i8008_run(s, prog, len, 200);
}

int main(void) {
    /* ── basic arithmetic ──────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x06, 0x01, 0x3E, 0x02, 0x80, 0x76};
        I8008Sim *s = i8008_new();
        size_t n = i8008_run(s, p, sizeof p, 100);
        ISO_CHECK_EQ_UINT(n, 4u);
        ISO_CHECK_EQ_UINT(i8008_a(s), 3u);
        ISO_CHECK(!i8008_flags(s).carry && !i8008_flags(s).zero &&
                  !i8008_flags(s).sign && i8008_flags(s).parity);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0xFF, 0xC4, 0x01, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x00u);
        ISO_CHECK(i8008_flags(s).carry && i8008_flags(s).zero &&
                  !i8008_flags(s).sign && i8008_flags(s).parity);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x00, 0xD4, 0x01, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0xFFu);
        ISO_CHECK(i8008_flags(s).carry && i8008_flags(s).sign &&
                  !i8008_flags(s).zero && i8008_flags(s).parity);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0xFE, 0xC4, 0x01, 0xCC, 0x01, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x00u);
        ISO_CHECK(i8008_flags(s).carry && i8008_flags(s).zero);
        i8008_free(s);
    }

    /* ── logical ───────────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x3E, 0xFF, 0xC4, 0x01, 0xA7, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK(!i8008_flags(s).carry);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x00u);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0xAB, 0xAF, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x00u);
        ISO_CHECK(i8008_flags(s).zero && !i8008_flags(s).carry &&
                  i8008_flags(s).parity);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x0F, 0x06, 0xF0, 0xB0, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0xFFu);
        ISO_CHECK(!i8008_flags(s).carry && i8008_flags(s).parity);
        i8008_free(s);
    }

    /* ── INR / DCR ─────────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x3E, 0xFF, 0xC4, 0x01, 0x06,
                                    0xFF, 0x00, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_b(s), 0x00u);
        ISO_CHECK(i8008_flags(s).zero && i8008_flags(s).carry);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x00, 0x39, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0xFFu);
        ISO_CHECK(i8008_flags(s).sign && !i8008_flags(s).zero &&
                  i8008_flags(s).parity);
        i8008_free(s);
    }

    /* ── rotates ───────────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x3E, 0x80, 0x02, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x01u);
        ISO_CHECK(i8008_flags(s).carry);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x01, 0x0A, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x80u);
        ISO_CHECK(i8008_flags(s).carry);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0xFF, 0x12, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0xFEu);
        ISO_CHECK(i8008_flags(s).carry);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0xFF, 0xC4, 0x01,
                                    0x3E, 0x01, 0x1A, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x80u);
        ISO_CHECK(i8008_flags(s).carry);
        i8008_free(s);
    }

    /* ── stack: call / return, nesting, RST ────────────────────────────────*/
    {
        uint8_t p[0x14];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x3E; p[1] = 0x00;
        p[2] = 0x7E; p[3] = 0x10; p[4] = 0x00; p[5] = 0x76;
        p[0x10] = 0x3E; p[0x11] = 0x2A; p[0x12] = 0x3F;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 42u);
        i8008_free(s);
    }
    {
        uint8_t p[0x50];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x3E; p[1] = 0x00;
        p[2] = 0x7E; p[3] = 0x20; p[4] = 0x00; p[5] = 0x76;
        p[0x20] = 0x7E; p[0x21] = 0x40; p[0x22] = 0x00; p[0x23] = 0x3F;
        p[0x40] = 0x3E; p[0x41] = 99; p[0x42] = 0x3F;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 99u);
        i8008_free(s);
    }
    {
        uint8_t p[0x20];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x1D; p[1] = 0x76;
        p[0x18] = 0x3E; p[0x19] = 77; p[0x1A] = 0x3F;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 77u);
        i8008_free(s);
    }

    /* ── memory via M ──────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x26, 0x00, 0x2E, 0x20, 0x36,
                                    0x55, 0x6E, 0x7D, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x55u);
        i8008_free(s);
    }

    /* ── conditional jumps ─────────────────────────────────────────────────*/
    {
        uint8_t p[0x14];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x3E; p[1] = 0x00; p[2] = 0xC4; p[3] = 0x00;
        p[4] = 0x4C; p[5] = 0x10; p[6] = 0x00;
        p[7] = 0x3E; p[8] = 99; p[9] = 0x76;
        p[0x10] = 0x3E; p[0x11] = 42; p[0x12] = 0x76;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 42u);
        i8008_free(s);
    }
    {
        uint8_t p[0x14];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x3E; p[1] = 0x01; p[2] = 0x4C; p[3] = 0x10; p[4] = 0x00;
        p[5] = 0x3E; p[6] = 99; p[7] = 0x76;
        p[0x10] = 0x3E; p[0x11] = 42; p[0x12] = 0x76;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 99u);
        i8008_free(s);
    }

    /* ── parity ────────────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x3E, 0x03, 0xF4, 0x00, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK(i8008_flags(s).parity);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x01, 0xF4, 0x00, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK(!i8008_flags(s).parity);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0xFF, 0xF4, 0x00, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK(i8008_flags(s).parity);
        i8008_free(s);
    }

    /* ── CMP ───────────────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x3E, 0x05, 0xFC, 0x05, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 5u);
        ISO_CHECK(i8008_flags(s).zero && !i8008_flags(s).carry);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x03, 0xFC, 0x05, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 3u);
        ISO_CHECK(i8008_flags(s).carry && !i8008_flags(s).zero);
        i8008_free(s);
    }

    /* ── I/O ports ─────────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x59, 0x76};
        I8008Sim *s = i8008_new();
        i8008_set_input_port(s, 3, 0xAB);
        i8008_run(s, p, sizeof p, 10);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0xABu);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x77, 0x22, 0x76};
        I8008Sim *s = i8008_new();
        i8008_run(s, p, sizeof p, 10);
        ISO_CHECK_EQ_UINT(i8008_get_output_port(s, 17), 0x77u);
        i8008_free(s);
    }

    /* ── abs / multiply / sbb / mov ────────────────────────────────────────*/
    {
        uint8_t p[0x40];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x3E; p[1] = 0xF6; p[2] = 0xF4; p[3] = 0x00;
        p[4] = 0x7E; p[5] = 0x20; p[6] = 0x00; p[7] = 0x76;
        p[0x20] = 0x50; p[0x21] = 0x30; p[0x22] = 0x00;
        p[0x23] = 0xEC; p[0x24] = 0xFF; p[0x25] = 0xC4; p[0x26] = 0x01;
        p[0x30] = 0x3F;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 10u);
        i8008_free(s);
    }
    {
        uint8_t p[20];
        I8008Sim *s = i8008_new();
        memset(p, 0, sizeof p);
        p[0] = 0x06; p[1] = 0x05; p[2] = 0x0E; p[3] = 0x04;
        p[4] = 0x3E; p[5] = 0x00; p[6] = 0x80; p[7] = 0x09;
        p[8] = 0x48; p[9] = 0x06; p[10] = 0x00; p[11] = 0x76;
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 20u);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x3E, 0x05, 0xC4, 0xFF, 0xDC, 0x01, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 2u);
        i8008_free(s);
    }
    {
        static const uint8_t p[] = {0x06, 0x42, 0x78, 0x76};
        I8008Sim *s = i8008_new();
        run(s, p, sizeof p);
        ISO_CHECK_EQ_UINT(i8008_a(s), 0x42u);
        i8008_free(s);
    }

    /* ── trace contents ────────────────────────────────────────────────────*/
    {
        static const uint8_t p[] = {0x3E, 0x05, 0x76};
        I8008Sim *s = i8008_new();
        I8008Trace t0, t1;
        size_t n = i8008_run(s, p, sizeof p, 100);
        ISO_CHECK_EQ_UINT(n, 2u);
        ISO_CHECK(i8008_trace(s, 0, &t0));
        ISO_CHECK_EQ_UINT(t0.address, 0u);
        ISO_CHECK_EQ_UINT(t0.a_before, 0u);
        ISO_CHECK_EQ_UINT(t0.a_after, 5u);
        ISO_CHECK(t0.raw_len == 2 && t0.raw[0] == 0x3E && t0.raw[1] == 0x05);
        ISO_CHECK(i8008_trace(s, 1, &t1));
        ISO_CHECK_EQ_UINT(t1.address, 2u);
        ISO_CHECK(strstr(t1.mnemonic, "HLT") != NULL);
        i8008_free(s);
    }

    return ISO_TEST_RESULT();
}
