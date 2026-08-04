/*
 * Tests for intel4004-simulator, mirroring the Rust crate's unit tests, using
 * the header-only iso_test.h harness (pure ISO C17).
 */
#include "iso_test.h"

#include "intel4004_simulator.h"

#include <stdlib.h> /* free */
#include <string.h> /* strstr */

/* Build a 4096-byte simulator, run `program`, and return it (caller frees).
 * Mirrors the Rust test helper `run_program`, which caps at 1000 steps. */
static I4004Sim *run_program(const uint8_t *program, size_t len) {
    I4004Sim *s = i4004_new(4096);
    i4004_run(s, program, len, 1000);
    return s;
}

int main(void) {
    /* ── NOP and HLT ────────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {0x00, 0x00, 0x01}; /* NOP, NOP, HLT */
        I4004Sim *s = i4004_new(4096);
        I4004Trace t;
        size_t n = i4004_run(s, prog, sizeof prog, 10);
        ISO_CHECK_EQ_UINT(n, 3);
        i4004_trace(s, 0, &t);
        ISO_CHECK_STR_EQ(t.mnemonic, "NOP");
        i4004_trace(s, 1, &t);
        ISO_CHECK_STR_EQ(t.mnemonic, "NOP");
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {0x01}; /* HLT */
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK(i4004_halted(s));
        i4004_free(s);
    }
    { /* step() on a halted CPU declines (returns 0) rather than panicking. */
        I4004Sim *s = i4004_new(4096);
        uint8_t prog[] = {0x01};
        i4004_run(s, prog, sizeof prog, 10);
        ISO_CHECK(i4004_halted(s));
        ISO_CHECK_EQ_INT(i4004_step(s, NULL), 0);
        i4004_free(s);
    }

    /* ── LDM ────────────────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(7), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 7);
        i4004_free(s);
    }
    {
        uint8_t n;
        for (n = 0; n <= 15; n++) {
            uint8_t prog[] = {i4004_encode_ldm(n), i4004_encode_hlt()};
            I4004Sim *s = run_program(prog, sizeof prog);
            ISO_CHECK_EQ_INT(i4004_accumulator(s), n);
            i4004_free(s);
        }
    }

    /* ── LD / XCH ───────────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(9), i4004_encode_xch(0),
                          i4004_encode_ld(0), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 9);
        i4004_free(s);
    }
    {
        /* A=5, R0=3, XCH R0 -> A=3, R0=5 */
        uint8_t prog[] = {i4004_encode_ldm(3), i4004_encode_xch(0),
                          i4004_encode_ldm(5), i4004_encode_xch(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 5);
        i4004_free(s);
    }

    /* ── ADD ────────────────────────────────────────────────────────────── */
    {
        /* A=1, R0=1, A=2, ADD -> 3 */
        uint8_t prog[] = {i4004_encode_ldm(1), i4004_encode_xch(0),
                          i4004_encode_ldm(2), i4004_encode_add(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        /* 9 + 5 = 14, no carry */
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_xch(0),
                          i4004_encode_ldm(9), i4004_encode_add(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 14);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        /* 9 + 8 = 17 -> A=1, carry */
        uint8_t prog[] = {i4004_encode_ldm(8), i4004_encode_xch(0),
                          i4004_encode_ldm(9), i4004_encode_add(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 1);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        /* ADD includes carry: STC then 4+4+1 = 9 */
        I4004Sim *s = i4004_new(4096);
        uint8_t prog[] = {i4004_encode_ldm(4), i4004_encode_xch(0),
                          i4004_encode_ldm(4), i4004_encode_stc(),
                          i4004_encode_add(0), i4004_encode_hlt()};
        i4004_load_program(s, prog, sizeof prog);
        while (!i4004_halted(s)) {
            i4004_step(s, NULL);
        }
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 9);
        i4004_free(s);
    }

    /* ── SUB (inverted-carry convention) ────────────────────────────────── */
    {
        /* 5 - 3 = 2, no borrow -> carry set */
        uint8_t prog[] = {i4004_encode_ldm(3), i4004_encode_xch(0),
                          i4004_encode_ldm(5), i4004_encode_sub(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 2);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        /* 3 - 5 = -2 -> wraps to 14, borrow -> carry clear */
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_xch(0),
                          i4004_encode_ldm(3), i4004_encode_sub(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 14);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        /* 0 - 1 = 15, borrow -> carry clear */
        uint8_t prog[] = {i4004_encode_ldm(1), i4004_encode_xch(0),
                          i4004_encode_ldm(0), i4004_encode_sub(0),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 15);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }

    /* ── INC ────────────────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(7), i4004_encode_xch(2),
                          i4004_encode_inc(2), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 2), 8);
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(15), i4004_encode_xch(0),
                          i4004_encode_inc(0), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 0);
        i4004_free(s);
    }
    {
        /* INC does not affect carry */
        I4004Sim *s = i4004_new(4096);
        uint8_t prog[] = {i4004_encode_ldm(15), i4004_encode_xch(0),
                          i4004_encode_stc(), i4004_encode_inc(0),
                          i4004_encode_hlt()};
        i4004_load_program(s, prog, sizeof prog);
        while (!i4004_halted(s)) {
            i4004_step(s, NULL);
        }
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 0);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }

    /* ── JUN ────────────────────────────────────────────────────────────── */
    {
        uint8_t lo, b1 = i4004_encode_jun(0x004, &lo);
        uint8_t prog[] = {b1, lo, i4004_encode_ldm(15), i4004_encode_hlt(),
                          i4004_encode_ldm(7), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 7);
        i4004_free(s);
    }

    /* ── JMS / BBL ──────────────────────────────────────────────────────── */
    {
        uint8_t lo, b1 = i4004_encode_jms(0x004, &lo);
        uint8_t prog[] = {b1, lo, i4004_encode_hlt(), 0x00,
                          i4004_encode_ldm(5), i4004_encode_bbl(3)};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        i4004_free(s);
    }
    {
        /* Two levels of nesting: main->0x10->0x20; returns BBL 7 then BBL 9. */
        uint8_t prog[256];
        uint8_t lo, b1;
        I4004Sim *s;
        memset(prog, 0, sizeof prog);
        b1 = i4004_encode_jms(0x010, &lo);
        prog[0] = b1;
        prog[1] = lo;
        prog[2] = i4004_encode_hlt();
        b1 = i4004_encode_jms(0x020, &lo);
        prog[0x010] = b1;
        prog[0x011] = lo;
        prog[0x012] = i4004_encode_bbl(9);
        prog[0x020] = i4004_encode_bbl(7);
        s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 9);
        i4004_free(s);
    }

    /* ── JCN ────────────────────────────────────────────────────────────── */
    {
        /* cond 0x4 (test zero); A=0 -> jump */
        uint8_t lo, b1 = i4004_encode_jcn(0x4, 0x05, &lo);
        uint8_t prog[] = {b1, lo, i4004_encode_ldm(15), i4004_encode_hlt(),
                          0x00, i4004_encode_ldm(1), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 1);
        i4004_free(s);
    }
    {
        /* cond 0x4; A=5 -> no jump */
        uint8_t lo, b1 = i4004_encode_jcn(0x4, 0x06, &lo);
        uint8_t prog[] = {i4004_encode_ldm(5), b1, lo, i4004_encode_ldm(2),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 2);
        i4004_free(s);
    }
    {
        /* cond 0xC (invert + test zero); A=5 (nonzero) -> jump */
        uint8_t lo, b1 = i4004_encode_jcn(0xC, 0x05, &lo);
        uint8_t prog[] = {i4004_encode_ldm(5), b1, lo, i4004_encode_ldm(15),
                          i4004_encode_hlt(), i4004_encode_ldm(1),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 1);
        i4004_free(s);
    }
    {
        /* cond 0x2 (test carry); carry set -> jump */
        uint8_t lo, b1 = i4004_encode_jcn(0x2, 0x05, &lo);
        uint8_t prog[] = {i4004_encode_stc(), b1, lo, i4004_encode_ldm(15),
                          i4004_encode_hlt(), i4004_encode_ldm(1),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 1);
        i4004_free(s);
    }

    /* ── ISZ ────────────────────────────────────────────────────────────── */
    {
        /* R0=14, ISZ loops until wrap to 0 */
        uint8_t lo, isz = i4004_encode_isz(0, 0x02, &lo);
        uint8_t prog[] = {i4004_encode_ldm(14), i4004_encode_xch(0), isz, lo,
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 0);
        i4004_free(s);
    }
    {
        /* R0=15, ISZ -> 0, falls through */
        uint8_t lo, isz = i4004_encode_isz(0, 0x10, &lo);
        uint8_t prog[] = {i4004_encode_ldm(15), i4004_encode_xch(0), isz, lo,
                          i4004_encode_ldm(7), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 0);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 7);
        i4004_free(s);
    }

    /* ── FIM / register pairs ───────────────────────────────────────────── */
    {
        uint8_t lo, b1 = i4004_encode_fim(0, 0xA3, &lo);
        uint8_t prog[] = {b1, lo, i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 0xA);
        ISO_CHECK_EQ_INT(i4004_register(s, 1), 0x3);
        i4004_free(s);
    }
    {
        uint8_t prog[17];
        uint8_t lo, b1, p;
        size_t idx = 0;
        I4004Sim *s;
        for (p = 0; p < 8; p++) {
            uint8_t val = (uint8_t)((p << 4) | (15 - p));
            b1 = i4004_encode_fim(p, val, &lo);
            prog[idx++] = b1;
            prog[idx++] = lo;
        }
        prog[idx++] = i4004_encode_hlt();
        s = run_program(prog, idx);
        for (p = 0; p < 8; p++) {
            uint8_t val = (uint8_t)((p << 4) | (15 - p));
            ISO_CHECK_EQ_INT(i4004_register(s, (size_t)p * 2),
                             (val >> 4) & 0xF);
            ISO_CHECK_EQ_INT(i4004_register(s, (size_t)p * 2 + 1), val & 0xF);
        }
        i4004_free(s);
    }
    { /* register-pair operations: FIM P3, 0xDE -> R6=D, R7=E */
        uint8_t lo, b1 = i4004_encode_fim(3, 0xDE, &lo);
        uint8_t prog[] = {b1, lo, i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 6), 0xD);
        ISO_CHECK_EQ_INT(i4004_register(s, 7), 0xE);
        i4004_free(s);
    }

    /* ── SRC / FIN / JIN ────────────────────────────────────────────────── */
    {
        uint8_t lo, b1 = i4004_encode_fim(0, 0x25, &lo);
        uint8_t prog[] = {b1, lo, i4004_encode_src(0), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_UINT(i4004_ram_register(s), 2);
        ISO_CHECK_EQ_UINT(i4004_ram_character(s), 5);
        i4004_free(s);
    }
    {
        /* FIN reads ROM[0x08] = 0xBC into pair 1 (R2,R3) */
        uint8_t lo, fim = i4004_encode_fim(0, 0x08, &lo);
        uint8_t prog[9];
        I4004Sim *s;
        memset(prog, 0, sizeof prog);
        prog[0] = fim;
        prog[1] = lo;
        prog[2] = i4004_encode_fin(1);
        prog[3] = i4004_encode_hlt();
        prog[8] = 0xBC;
        s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 2), 0xB);
        ISO_CHECK_EQ_INT(i4004_register(s, 3), 0xC);
        i4004_free(s);
    }
    {
        /* JIN P0 with pair 0 = 0x05 jumps to addr 5 */
        uint8_t lo, fim = i4004_encode_fim(0, 0x05, &lo);
        uint8_t prog[] = {fim, lo, i4004_encode_jin(0), i4004_encode_ldm(15),
                          i4004_encode_hlt(), i4004_encode_ldm(3),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        i4004_free(s);
    }

    /* ── RAM: WRM/RDM, DCL banks ────────────────────────────────────────── */
    {
        uint8_t lo, fim = i4004_encode_fim(0, 0x00, &lo);
        uint8_t prog[] = {fim, lo, i4004_encode_src(0), i4004_encode_ldm(7),
                          i4004_encode_wrm(), i4004_encode_ldm(0),
                          i4004_encode_rdm(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 7);
        i4004_free(s);
    }
    {
        /* Write 5 to bank 0, 9 to bank 2, read both back. */
        uint8_t lo, fim = i4004_encode_fim(0, 0x00, &lo);
        uint8_t prog[] = {
            fim, lo, i4004_encode_src(0), i4004_encode_ldm(0),
            i4004_encode_dcl(), i4004_encode_ldm(5), i4004_encode_wrm(),
            i4004_encode_ldm(2), i4004_encode_dcl(), i4004_encode_ldm(9),
            i4004_encode_wrm(), i4004_encode_ldm(0), i4004_encode_dcl(),
            i4004_encode_rdm(), i4004_encode_xch(2), i4004_encode_ldm(2),
            i4004_encode_dcl(), i4004_encode_rdm(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 9);
        ISO_CHECK_EQ_INT(i4004_register(s, 2), 5);
        i4004_free(s);
    }

    /* ── RAM status: WR0-WR3 / RD0-RD3 ──────────────────────────────────── */
    {
        uint8_t lo, fim = i4004_encode_fim(0, 0x00, &lo);
        uint8_t prog[] = {
            fim, lo, i4004_encode_src(0), i4004_encode_ldm(1),
            i4004_encode_wr0(), i4004_encode_ldm(2), i4004_encode_wr1(),
            i4004_encode_ldm(3), i4004_encode_wr2(), i4004_encode_ldm(4),
            i4004_encode_wr3(), i4004_encode_rd0(), i4004_encode_xch(4),
            i4004_encode_rd1(), i4004_encode_xch(5), i4004_encode_rd2(),
            i4004_encode_xch(6), i4004_encode_rd3(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 4), 1);
        ISO_CHECK_EQ_INT(i4004_register(s, 5), 2);
        ISO_CHECK_EQ_INT(i4004_register(s, 6), 3);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 4);
        i4004_free(s);
    }

    /* ── ROM port: WRR / RDR ────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(11), i4004_encode_wrr(),
                          i4004_encode_ldm(0), i4004_encode_rdr(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 11);
        i4004_free(s);
    }

    /* ── WMP output port ────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(0), i4004_encode_dcl(),
                          i4004_encode_ldm(13), i4004_encode_wmp(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_ram_output(s, 0), 13);
        i4004_free(s);
    }

    /* ── ADM / SBM ──────────────────────────────────────────────────────── */
    {
        uint8_t lo, fim = i4004_encode_fim(0, 0x00, &lo);
        uint8_t prog[] = {fim, lo, i4004_encode_src(0), i4004_encode_ldm(6),
                          i4004_encode_wrm(), i4004_encode_ldm(3),
                          i4004_encode_adm(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 9);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t lo, fim = i4004_encode_fim(0, 0x00, &lo);
        uint8_t prog[] = {fim, lo, i4004_encode_src(0), i4004_encode_ldm(3),
                          i4004_encode_wrm(), i4004_encode_ldm(7),
                          i4004_encode_sbm(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 4);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }

    /* ── Accumulator group: CLB, CLC, IAC, CMC, CMA ─────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(15), i4004_encode_stc(),
                          i4004_encode_clb(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(7), i4004_encode_stc(),
                          i4004_encode_clc(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 7);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(4), i4004_encode_iac(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 5);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(15), i4004_encode_iac(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_cmc(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_stc(), i4004_encode_cmc(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_cma(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 10);
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(0), i4004_encode_cma(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 15);
        i4004_free(s);
    }

    /* ── RAL / RAR ──────────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_ral(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 10);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_stc(),
                          i4004_encode_ral(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 11);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(8), i4004_encode_ral(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(6), i4004_encode_rar(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(6), i4004_encode_stc(),
                          i4004_encode_rar(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 11);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(1), i4004_encode_rar(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }

    /* ── TCC / DAC / TCS / STC ──────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_stc(), i4004_encode_tcc(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 1);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_tcc(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_dac(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 4);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(0), i4004_encode_dac(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 15);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_stc(), i4004_encode_tcs(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 10);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_tcs(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 9);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_stc(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }

    /* ── DAA ────────────────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_daa(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 5);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        /* 8+5=13, DAA adds 6 -> A=3, carry */
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_xch(0),
                          i4004_encode_ldm(8), i4004_encode_add(0),
                          i4004_encode_daa(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        /* A=2, carry set -> DAA adds 6 -> 8, carry stays set */
        uint8_t prog[] = {i4004_encode_ldm(2), i4004_encode_stc(),
                          i4004_encode_daa(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 8);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }

    /* ── KBP (exhaustive one-hot decode) ────────────────────────────────── */
    {
        static const uint8_t expected[16] = {0, 1, 2, 15, 3,  15, 15, 15,
                                             4, 15, 15, 15, 15, 15, 15, 15};
        uint8_t input;
        for (input = 0; input < 16; input++) {
            uint8_t prog[] = {i4004_encode_ldm(input), i4004_encode_kbp(),
                              i4004_encode_hlt()};
            I4004Sim *s = run_program(prog, sizeof prog);
            ISO_CHECK_EQ_INT(i4004_accumulator(s), expected[input]);
            i4004_free(s);
        }
    }

    /* ── DCL bank selection ─────────────────────────────────────────────── */
    {
        uint8_t bank;
        for (bank = 0; bank < 4; bank++) {
            uint8_t prog[] = {i4004_encode_ldm(bank), i4004_encode_dcl(),
                              i4004_encode_hlt()};
            I4004Sim *s = run_program(prog, sizeof prog);
            ISO_CHECK_EQ_UINT(i4004_ram_bank(s), bank);
            i4004_free(s);
        }
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(7), i4004_encode_dcl(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_UINT(i4004_ram_bank(s), 3);
        i4004_free(s);
    }

    /* ── WPM is a no-op ─────────────────────────────────────────────────── */
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_wpm(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 5);
        i4004_free(s);
    }

    /* ── reset clears state ─────────────────────────────────────────────── */
    {
        /* Dirty a broad range of state, then reset and confirm it all zeroes. */
        uint8_t lo, fim = i4004_encode_fim(0, 0x25, &lo);
        uint8_t prog[] = {i4004_encode_ldm(15), i4004_encode_xch(0),
                          i4004_encode_stc(), fim, lo, i4004_encode_src(0),
                          i4004_encode_ldm(5), i4004_encode_wrm(),
                          i4004_encode_hlt()};
        I4004Sim *s = i4004_new(4096);
        i4004_run(s, prog, sizeof prog, 100);
        ISO_CHECK(i4004_register(s, 0) != 0 || i4004_carry(s) ||
                  i4004_ram(s, 0, 2, 5) != 0);
        i4004_reset(s);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(!i4004_carry(s));
        ISO_CHECK_EQ_INT(i4004_register(s, 0), 0);
        ISO_CHECK_EQ_UINT(i4004_pc(s), 0);
        ISO_CHECK(!i4004_halted(s));
        ISO_CHECK_EQ_INT(i4004_hw_stack(s, 0), 0);
        ISO_CHECK_EQ_UINT(i4004_stack_pointer(s), 0);
        ISO_CHECK_EQ_INT(i4004_ram(s, 0, 2, 5), 0);
        ISO_CHECK_EQ_UINT(i4004_ram_bank(s), 0);
        ISO_CHECK_EQ_UINT(i4004_ram_register(s), 0);
        ISO_CHECK_EQ_UINT(i4004_ram_character(s), 0);
        ISO_CHECK_EQ_INT(i4004_rom_port(s), 0);
        i4004_free(s);
    }

    /* ── trace records raw2 / two-byte detection / unknown ──────────────── */
    {
        uint8_t lo, b1 = i4004_encode_jun(0x004, &lo);
        uint8_t prog[] = {b1, lo, 0, 0, i4004_encode_hlt()};
        I4004Sim *s = i4004_new(4096);
        I4004Trace t;
        i4004_load_program(s, prog, sizeof prog);
        i4004_step(s, &t);
        ISO_CHECK_EQ_INT(t.raw, b1);
        ISO_CHECK(t.has_raw2);
        ISO_CHECK_EQ_INT(t.raw2, lo);
        i4004_free(s);
    }
    {
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_hlt()};
        I4004Sim *s = i4004_new(4096);
        I4004Trace t;
        i4004_load_program(s, prog, sizeof prog);
        i4004_step(s, &t);
        ISO_CHECK(!t.has_raw2);
        i4004_free(s);
    }
    {
        uint8_t prog[] = {0xFE, i4004_encode_hlt()};
        I4004Sim *s = i4004_new(4096);
        I4004Trace t;
        i4004_load_program(s, prog, sizeof prog);
        i4004_step(s, &t);
        ISO_CHECK(strstr(t.mnemonic, "UNKNOWN") != NULL);
        i4004_free(s);
    }

    /* ── End-to-end programs ────────────────────────────────────────────── */
    {
        /* x = 1 + 2 through the accumulator, stored in R1. */
        uint8_t prog[] = {i4004_encode_ldm(1), i4004_encode_xch(0),
                          i4004_encode_ldm(2), i4004_encode_add(0),
                          i4004_encode_xch(1), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_register(s, 1), 3);
        i4004_free(s);
    }
    {
        /* Countdown from 5 to 0 using DAC + JCN invert-zero loop. */
        uint8_t lo, jcn = i4004_encode_jcn(0xC, 0x01, &lo);
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_dac(), jcn, lo,
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        i4004_free(s);
    }
    {
        /* BCD addition 8 + 5 = 13 -> carry=1, digit=3. */
        uint8_t prog[] = {i4004_encode_ldm(5), i4004_encode_xch(0),
                          i4004_encode_ldm(8), i4004_encode_clc(),
                          i4004_encode_add(0), i4004_encode_daa(),
                          i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        ISO_CHECK(i4004_carry(s));
        i4004_free(s);
    }
    {
        /* Subroutine that doubles R0=6 into R1, result 12 in A. */
        uint8_t prog[256];
        uint8_t lo, b1;
        I4004Sim *s;
        memset(prog, 0, sizeof prog);
        prog[0] = i4004_encode_ldm(6);
        prog[1] = i4004_encode_xch(0);
        b1 = i4004_encode_jms(0x010, &lo);
        prog[2] = b1;
        prog[3] = lo;
        prog[4] = i4004_encode_ld(1);
        prog[5] = i4004_encode_hlt();
        prog[0x010] = i4004_encode_ld(0);
        prog[0x011] = i4004_encode_add(0);
        prog[0x012] = i4004_encode_xch(1);
        prog[0x013] = i4004_encode_bbl(0);
        s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 12);
        i4004_free(s);
    }
    {
        /* Store 1,3,5 in RAM chars 0,1,2, read back char 1 (=3). */
        uint8_t prog[64];
        size_t idx = 0;
        uint8_t lo, b1, i;
        static const uint8_t values[3] = {1, 3, 5};
        I4004Sim *s;
        for (i = 0; i < 3; i++) {
            b1 = i4004_encode_fim(0, i, &lo);
            prog[idx++] = b1;
            prog[idx++] = lo;
            prog[idx++] = i4004_encode_src(0);
            prog[idx++] = i4004_encode_ldm(values[i]);
            prog[idx++] = i4004_encode_wrm();
        }
        b1 = i4004_encode_fim(0, 1, &lo);
        prog[idx++] = b1;
        prog[idx++] = lo;
        prog[idx++] = i4004_encode_src(0);
        prog[idx++] = i4004_encode_rdm();
        prog[idx++] = i4004_encode_hlt();
        s = run_program(prog, idx);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 3);
        i4004_free(s);
    }
    {
        /* ISZ loop summing 1+2+3 = 6. */
        uint8_t lo1, fim = i4004_encode_fim(0, (uint8_t)(13 << 4), &lo1);
        uint8_t lo2, isz = i4004_encode_isz(0, 0x04, &lo2);
        uint8_t prog[] = {fim, lo1, i4004_encode_ldm(0), i4004_encode_xch(2),
                          i4004_encode_inc(1), i4004_encode_ld(2),
                          i4004_encode_add(1), i4004_encode_xch(2), isz, lo2,
                          i4004_encode_ld(2), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 6);
        i4004_free(s);
    }
    {
        /* Rotate left twice: 1 -> 2 -> 4. */
        uint8_t prog[] = {i4004_encode_ldm(1), i4004_encode_ral(),
                          i4004_encode_ral(), i4004_encode_hlt()};
        I4004Sim *s = run_program(prog, sizeof prog);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 4);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }
    {
        /* Stack wrapping: nest 4 deep in a 3-slot stack; just must not crash. */
        uint8_t prog[256];
        uint8_t lo, b1;
        I4004Sim *s = i4004_new(4096);
        memset(prog, 0, sizeof prog);
        b1 = i4004_encode_jms(0x20, &lo);
        prog[0x10] = b1;
        prog[0x11] = lo;
        prog[0x12] = i4004_encode_bbl(0);
        b1 = i4004_encode_jms(0x30, &lo);
        prog[0x20] = b1;
        prog[0x21] = lo;
        prog[0x22] = i4004_encode_bbl(0);
        b1 = i4004_encode_jms(0x40, &lo);
        prog[0x30] = b1;
        prog[0x31] = lo;
        prog[0x32] = i4004_encode_bbl(0);
        prog[0x40] = i4004_encode_bbl(0);
        b1 = i4004_encode_jms(0x10, &lo);
        prog[0x00] = b1;
        prog[0x01] = lo;
        prog[0x02] = i4004_encode_hlt();
        i4004_run(s, prog, sizeof prog, 100);
        ISO_CHECK(1); /* survived without out-of-bounds access */
        i4004_free(s);
    }

    /* ── Encoding roundtrip ─────────────────────────────────────────────── */
    {
        uint8_t lo;
        ISO_CHECK_EQ_INT(i4004_encode_nop(), 0x00);
        ISO_CHECK_EQ_INT(i4004_encode_hlt(), 0x01);
        ISO_CHECK_EQ_INT(i4004_encode_ldm(5), 0xD5);
        ISO_CHECK_EQ_INT(i4004_encode_ld(3), 0xA3);
        ISO_CHECK_EQ_INT(i4004_encode_xch(7), 0xB7);
        ISO_CHECK_EQ_INT(i4004_encode_add(2), 0x82);
        ISO_CHECK_EQ_INT(i4004_encode_sub(4), 0x94);
        ISO_CHECK_EQ_INT(i4004_encode_inc(6), 0x66);
        ISO_CHECK_EQ_INT(i4004_encode_bbl(1), 0xC1);
        ISO_CHECK_EQ_INT(i4004_encode_jcn(0x4, 0x10, &lo), 0x14);
        ISO_CHECK_EQ_INT(lo, 0x10);
        ISO_CHECK_EQ_INT(i4004_encode_fim(2, 0xAB, &lo), 0x24);
        ISO_CHECK_EQ_INT(lo, 0xAB);
        ISO_CHECK_EQ_INT(i4004_encode_src(1), 0x23);
        ISO_CHECK_EQ_INT(i4004_encode_fin(3), 0x36);
        ISO_CHECK_EQ_INT(i4004_encode_jin(3), 0x37);
        ISO_CHECK_EQ_INT(i4004_encode_jun(0x123, &lo), 0x41);
        ISO_CHECK_EQ_INT(lo, 0x23);
        ISO_CHECK_EQ_INT(i4004_encode_jms(0x456, &lo), 0x54);
        ISO_CHECK_EQ_INT(lo, 0x56);
        ISO_CHECK_EQ_INT(i4004_encode_isz(5, 0x20, &lo), 0x75);
        ISO_CHECK_EQ_INT(lo, 0x20);
        ISO_CHECK_EQ_INT(i4004_encode_wrm(), 0xE0);
        ISO_CHECK_EQ_INT(i4004_encode_wmp(), 0xE1);
        ISO_CHECK_EQ_INT(i4004_encode_wrr(), 0xE2);
        ISO_CHECK_EQ_INT(i4004_encode_wpm(), 0xE3);
        ISO_CHECK_EQ_INT(i4004_encode_wr0(), 0xE4);
        ISO_CHECK_EQ_INT(i4004_encode_wr1(), 0xE5);
        ISO_CHECK_EQ_INT(i4004_encode_wr2(), 0xE6);
        ISO_CHECK_EQ_INT(i4004_encode_wr3(), 0xE7);
        ISO_CHECK_EQ_INT(i4004_encode_sbm(), 0xE8);
        ISO_CHECK_EQ_INT(i4004_encode_rdm(), 0xE9);
        ISO_CHECK_EQ_INT(i4004_encode_rdr(), 0xEA);
        ISO_CHECK_EQ_INT(i4004_encode_adm(), 0xEB);
        ISO_CHECK_EQ_INT(i4004_encode_rd0(), 0xEC);
        ISO_CHECK_EQ_INT(i4004_encode_rd1(), 0xED);
        ISO_CHECK_EQ_INT(i4004_encode_rd2(), 0xEE);
        ISO_CHECK_EQ_INT(i4004_encode_rd3(), 0xEF);
        ISO_CHECK_EQ_INT(i4004_encode_clb(), 0xF0);
        ISO_CHECK_EQ_INT(i4004_encode_clc(), 0xF1);
        ISO_CHECK_EQ_INT(i4004_encode_iac(), 0xF2);
        ISO_CHECK_EQ_INT(i4004_encode_cmc(), 0xF3);
        ISO_CHECK_EQ_INT(i4004_encode_cma(), 0xF4);
        ISO_CHECK_EQ_INT(i4004_encode_ral(), 0xF5);
        ISO_CHECK_EQ_INT(i4004_encode_rar(), 0xF6);
        ISO_CHECK_EQ_INT(i4004_encode_tcc(), 0xF7);
        ISO_CHECK_EQ_INT(i4004_encode_dac(), 0xF8);
        ISO_CHECK_EQ_INT(i4004_encode_tcs(), 0xF9);
        ISO_CHECK_EQ_INT(i4004_encode_stc(), 0xFA);
        ISO_CHECK_EQ_INT(i4004_encode_daa(), 0xFB);
        ISO_CHECK_EQ_INT(i4004_encode_kbp(), 0xFC);
        ISO_CHECK_EQ_INT(i4004_encode_dcl(), 0xFD);
    }

    /* ── run() resets between programs ──────────────────────────────────── */
    {
        I4004Sim *s = i4004_new(4096);
        uint8_t p1[] = {i4004_encode_ldm(15), i4004_encode_stc(),
                        i4004_encode_hlt()};
        uint8_t p2[] = {i4004_encode_hlt()};
        i4004_run(s, p1, sizeof p1, 10);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 15);
        ISO_CHECK(i4004_carry(s));
        i4004_run(s, p2, sizeof p2, 10);
        ISO_CHECK_EQ_INT(i4004_accumulator(s), 0);
        ISO_CHECK(!i4004_carry(s));
        i4004_free(s);
    }

    /* ── two-byte detection predicate (via trace has_raw2) ──────────────── */
    {
        /* SRC (0x2 odd) must be single-byte; FIM (0x2 even) two-byte. */
        I4004Sim *s = i4004_new(4096);
        I4004Trace t;
        uint8_t prog[] = {i4004_encode_src(0), i4004_encode_hlt()};
        i4004_load_program(s, prog, sizeof prog);
        i4004_step(s, &t);
        ISO_CHECK(!t.has_raw2);
        i4004_free(s);
    }

    return ISO_TEST_RESULT();
}
