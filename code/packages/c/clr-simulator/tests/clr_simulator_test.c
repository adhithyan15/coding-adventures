/*
 * Tests for clr-simulator, using the header-only iso_test.h harness (pure ISO).
 * The vectors mirror the Rust crate's unit tests, plus extra coverage for the
 * bounds-safety this C port adds (untrusted-bytecode operand/index checks).
 */
#include "iso_test.h"

#include <stddef.h>
#include <stdint.h>

#include "clr_simulator.h"

/* Convenience: the integer value of the current stack top (asserts it is an
 * int). Returns INT32 sentinel on failure so a bad case is visible. */
static int32_t top_int(const ClrSimulator *sim) {
    ClrSlot s;
    if (!clr_stack_top(sim, &s) || !s.present || s.value.kind != CLR_INT) {
        return -999999;
    }
    return s.value.i;
}

int main(void) {
    /* ── clr_simulator_math: 1 + 2 -> local 0, reload, ret ────────────────── */
    {
        /* ldc.i4 1; ldc.i4 2; add; stloc.0; ldloc.0; ret */
        static const uint8_t prog[] = {0x17, 0x18, 0x58, 0x0A, 0x06, 0x2A};
        ClrSimulator *sim = clr_new();
        size_t steps = 0;
        ClrSlot loc;
        ISO_CHECK(sim != NULL);
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 16), CLR_OK);
        ISO_CHECK_EQ_INT(clr_run(sim, 100, &steps), CLR_OK);
        ISO_CHECK_EQ_UINT(steps, 6u);
        ISO_CHECK(clr_halted(sim));
        ISO_CHECK(clr_local_at(sim, 0, &loc));
        ISO_CHECK(loc.present && loc.value.kind == CLR_INT);
        ISO_CHECK_EQ_INT(loc.value.i, 3);
        clr_free(sim);
    }

    /* ── clr_div_by_zero: division by zero is a status, not a crash ───────── */
    {
        /* ldc.i4 1; ldc.i4 0; div */
        static const uint8_t prog[] = {0x17, 0x16, 0x5B};
        ClrSimulator *sim = clr_new();
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_run(sim, 100, NULL), CLR_ERR_DIVIDE_BY_ZERO);
        clr_free(sim);
    }

    /* ── clr_extended_opcodes: 10 cgt 5 == 1 ──────────────────────────────── */
    {
        /* ldc.i4.s 10; ldc.i4 5; FE CGT */
        static const uint8_t prog[] = {0x1F, 10, 0x1B, CLR_OP_PREFIX_FE,
                                       CLR_CGT_BYTE};
        ClrSimulator *sim = clr_new();
        size_t steps = 0;
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        /* Runs 3 steps then falls off the end (no ret) -> PC_OUT_OF_RANGE. */
        ISO_CHECK_EQ_INT(clr_run(sim, 100, &steps), CLR_ERR_PC_OUT_OF_RANGE);
        ISO_CHECK_EQ_UINT(steps, 3u);
        ISO_CHECK_EQ_INT(top_int(sim), 1);
        clr_free(sim);
    }

    /* ── clr_branching_zero: brfalse.s skips the ldc.i4 1000 ──────────────── */
    {
        /* ldc.i4 0; brfalse.s +5; ldc.i4 1000; ldc.i4.s 10 */
        static const uint8_t prog[] = {0x16, 0x2C, 5,   0x20, 0xE8,
                                       0x03, 0x00, 0x00, 0x1F, 10};
        ClrSimulator *sim = clr_new();
        size_t steps = 0;
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_run(sim, 100, &steps), CLR_ERR_PC_OUT_OF_RANGE);
        /* push 0, branch, push 10 -> 3 executed steps; top is 10, not 1000. */
        ISO_CHECK_EQ_UINT(steps, 3u);
        ISO_CHECK_EQ_INT(top_int(sim), 10);
        clr_free(sim);
    }

    /* ── clr_object_array_cons_roundtrip: newarr, box/stelem, ldelem ──────── */
    {
        static const uint8_t prog[] = {
            0x18,                   /* ldc.i4 2                    */
            0x8D, 0,    0,   0,   0, /* newarr <type>              */
            0x25,                   /* dup                         */
            0x16,                   /* ldc.i4 0                    */
            0x1D,                   /* ldc.i4 7                    */
            0x8C, 0,    0,   0,   0, /* box <type>                 */
            0xA4,                   /* stelem.ref  arr[0] = 7      */
            0x25,                   /* dup                         */
            0x17,                   /* ldc.i4 1                    */
            0x1F, 9,                /* ldc.i4.s 9                  */
            0x8C, 0,    0,   0,   0, /* box <type>                 */
            0xA4,                   /* stelem.ref  arr[1] = 9      */
            0x16,                   /* ldc.i4 0                    */
            0xA2,                   /* ldelem.ref  -> arr[0]       */
            0xA5, 0,    0,   0,   0  /* unbox.any                   */
        };
        ClrSimulator *sim = clr_new();
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        (void)clr_run(sim, 100, NULL); /* falls off the end after 15 steps */
        ISO_CHECK_EQ_INT(top_int(sim), 7);
        clr_free(sim);
    }

    /* ── clr_null_is_falsy: ldnull pushes Ref(None) ───────────────────────── */
    {
        static const uint8_t prog[] = {0x14}; /* ldnull */
        ClrSimulator *sim = clr_new();
        ClrSlot s;
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_step(sim), CLR_OK);
        ISO_CHECK(clr_stack_at(sim, 0, &s));
        ISO_CHECK(s.present && s.value.kind == CLR_REF && s.value.ref_some == 0);
        clr_free(sim);
    }

    /* ── clr_halted: stepping a halted machine errors ─────────────────────── */
    {
        static const uint8_t prog[] = {0x2A}; /* ret (no frame -> halt) */
        ClrSimulator *sim = clr_new();
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_step(sim), CLR_OK);
        ISO_CHECK(clr_halted(sim));
        ISO_CHECK_EQ_INT(clr_step(sim), CLR_ERR_HALTED);
        clr_free(sim);
    }

    /* ── method call: entry calls a 2-arg adder, result on the shared stack ─ */
    {
        /* entry (method 0): ldc.i4.s 20; ldc.i4.s 22; call #2; ret */
        static const uint8_t entry[] = {0x1F, 20,   0x1F, 22,  0x28,
                                        0x02, 0x00, 0x00, 0x06, 0x2A};
        /* adder (method 1, ordinal 2): ldarg.0; ldarg.1; add; ret */
        static const uint8_t adder[] = {0x02, 0x03, 0x58, 0x2A};
        ClrMethod methods[2];
        ClrSimulator *sim = clr_new();
        methods[0].body = entry;
        methods[0].body_len = sizeof entry;
        methods[0].num_locals = 0;
        methods[0].num_args = 0;
        methods[1].body = adder;
        methods[1].body_len = sizeof adder;
        methods[1].num_locals = 0;
        methods[1].num_args = 2;
        ISO_CHECK_EQ_INT(clr_load_program(sim, methods, 2, 0), CLR_OK);
        ISO_CHECK_EQ_INT(clr_run(sim, 100, NULL), CLR_OK);
        ISO_CHECK(clr_halted(sim));
        ISO_CHECK_EQ_INT(top_int(sim), 42);
        clr_free(sim);
    }

    /* ── bounds safety: a truncated ldc.i4 operand errors, no OOB read ────── */
    {
        static const uint8_t prog[] = {0x20}; /* ldc.i4 with no 4-byte operand */
        ClrSimulator *sim = clr_new();
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_step(sim), CLR_ERR_BYTECODE_OVERRUN);
        clr_free(sim);
    }

    /* ── unknown opcode is rejected ───────────────────────────────────────── */
    {
        static const uint8_t prog[] = {0x99};
        ClrSimulator *sim = clr_new();
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_step(sim), CLR_ERR_UNKNOWN_OPCODE);
        clr_free(sim);
    }

    /* ── stelem.ref index out of range is a status, not a crash ───────────── */
    {
        /* ldc.i4 1; newarr; dup; ldc.i4 5; ldc.i4 7; stelem.ref (5 >= 1) */
        static const uint8_t prog[] = {0x17, 0x8D, 0, 0, 0, 0, 0x25,
                                       0x1B, 0x1D, 0xA4};
        ClrSimulator *sim = clr_new();
        ISO_CHECK_EQ_INT(clr_load(sim, prog, sizeof prog, 4), CLR_OK);
        ISO_CHECK_EQ_INT(clr_run(sim, 100, NULL), CLR_ERR_INDEX_OUT_OF_RANGE);
        clr_free(sim);
    }

    /* ── encoding helpers ─────────────────────────────────────────────────── */
    {
        uint8_t buf[5];
        ISO_CHECK_EQ_UINT(clr_encode_ldc_i4(5, buf), 1u);
        ISO_CHECK_EQ_UINT(buf[0], 0x1Bu); /* LDC_I4_0 + 5 */
        ISO_CHECK_EQ_UINT(clr_encode_ldc_i4(100, buf), 2u);
        ISO_CHECK_EQ_UINT(buf[0], CLR_OP_LDC_I4_S);
        ISO_CHECK_EQ_UINT(buf[1], 100u);
        ISO_CHECK_EQ_UINT(clr_encode_ldc_i4(1000, buf), 5u);
        ISO_CHECK_EQ_UINT(buf[0], CLR_OP_LDC_I4);
        ISO_CHECK_EQ_UINT(buf[1], 0xE8u);
        ISO_CHECK_EQ_UINT(buf[2], 0x03u);
        ISO_CHECK_EQ_UINT(clr_encode_ldc_i4(-1, buf), 2u);
        ISO_CHECK_EQ_UINT(buf[0], CLR_OP_LDC_I4_S);
        ISO_CHECK_EQ_UINT(buf[1], 0xFFu);

        ISO_CHECK_EQ_UINT(clr_encode_stloc(0, buf), 1u);
        ISO_CHECK_EQ_UINT(buf[0], CLR_OP_STLOC_0);
        ISO_CHECK_EQ_UINT(clr_encode_stloc(5, buf), 2u);
        ISO_CHECK_EQ_UINT(buf[0], CLR_OP_STLOC_S);
        ISO_CHECK_EQ_UINT(buf[1], 5u);

        ISO_CHECK_EQ_UINT(clr_encode_ldloc(2, buf), 1u);
        ISO_CHECK_EQ_UINT(buf[0], 0x08u); /* LDLOC_0 + 2 */
        ISO_CHECK_EQ_UINT(clr_encode_ldloc(9, buf), 2u);
        ISO_CHECK_EQ_UINT(buf[0], CLR_OP_LDLOC_S);
        ISO_CHECK_EQ_UINT(buf[1], 9u);
    }

    return ISO_TEST_RESULT();
}
