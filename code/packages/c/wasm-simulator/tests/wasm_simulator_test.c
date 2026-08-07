/*
 * Tests for the C wasm-simulator, using the header-only iso_test.h harness (pure
 * ISO). The full-program vector mirrors the Rust crate's own test; the error
 * cases replace the Rust panics with status codes.
 */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "wasm_simulator.h"

int main(void) {
    /* ── decode: variable-length instructions ───────────────────────────── */
    {
        uint8_t code[5] = {WASM_OP_I32_CONST, 42, 0, 0, 0};
        WasmInstruction inst;
        ISO_CHECK(wasm_decode(code, 5, 0, &inst) == WASM_OK);
        ISO_CHECK_STR_EQ(inst.mnemonic, "i32.const");
        ISO_CHECK(inst.has_operand && inst.operand == 42);
        ISO_CHECK_EQ_UINT(inst.size, 5u);

        uint8_t add[1] = {WASM_OP_I32_ADD};
        ISO_CHECK(wasm_decode(add, 1, 0, &inst) == WASM_OK);
        ISO_CHECK_STR_EQ(inst.mnemonic, "i32.add");
        ISO_CHECK_EQ_UINT(inst.size, 1u);

        /* A truncated i32.const (only the opcode) is reported, not read OOB. */
        uint8_t trunc[1] = {WASM_OP_I32_CONST};
        ISO_CHECK(wasm_decode(trunc, 1, 0, &inst) == WASM_ERR_TRUNCATED);
        /* An unknown opcode. */
        uint8_t bad[1] = {0xFF};
        ISO_CHECK(wasm_decode(bad, 1, 0, &inst) == WASM_ERR_UNKNOWN_OPCODE);
    }

    /* ── full program: push 1, push 2, add, set 0, get 0, push 5, sub, end ─ */
    {
        WasmSimulator *sim = wasm_sim_new(4);
        ISO_CHECK(sim != NULL);
        WasmProgram p;
        wasm_program_init(&p);
        ISO_CHECK(wasm_emit_i32_const(&p, 1) == 0);
        ISO_CHECK(wasm_emit_i32_const(&p, 2) == 0);
        wasm_emit_i32_add(&p);
        wasm_emit_local_set(&p, 0);
        wasm_emit_local_get(&p, 0);
        wasm_emit_i32_const(&p, 5);
        wasm_emit_i32_sub(&p);
        wasm_emit_end(&p);

        size_t plen;
        const uint8_t *bytes = wasm_program_bytes(&p, &plen);
        ISO_CHECK_EQ_UINT(plen, 22u); /* 5+5+1+2+2+5+1+1 */

        WasmStepTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(wasm_sim_run(sim, bytes, plen, 1000, &traces, &count) ==
                  WASM_OK);
        ISO_CHECK_EQ_UINT(count, 8u);

        /* Final machine state. */
        size_t nlocals, nstack;
        const int32_t *locals = wasm_sim_locals(sim, &nlocals);
        const int32_t *stack = wasm_sim_stack(sim, &nstack);
        ISO_CHECK(locals[0] == 3);
        ISO_CHECK_EQ_UINT(nstack, 1u);
        ISO_CHECK(stack[0] == -2);                     /* 3 - 5 = -2 */
        ISO_CHECK((uint32_t)stack[0] == 4294967294u);  /* -2 as u32 */
        ISO_CHECK(wasm_sim_halted(sim));
        ISO_CHECK_EQ_UINT(wasm_sim_cycle(sim), 8u);

        /* A few trace details. */
        ISO_CHECK_STR_EQ(traces[0].description, "push 1");
        ISO_CHECK(traces[0].n_stack_before == 0 && traces[0].n_stack_after == 1);
        ISO_CHECK(traces[0].stack_after[0] == 1);
        ISO_CHECK_STR_EQ(traces[2].description, "pop 2 and 1, push 3"); /* add */
        ISO_CHECK(traces[2].stack_after[0] == 3);
        ISO_CHECK_STR_EQ(traces[6].description,
                         "pop 5 and 3, push -2"); /* sub */
        ISO_CHECK(traces[7].halted);              /* end */
        ISO_CHECK_STR_EQ(traces[7].description, "halt");

        wasm_traces_free(traces, count);
        wasm_program_free(&p);
        wasm_sim_free(sim);
    }

    /* ── stepping a halted VM is reported (the Rust panic) ──────────────── */
    {
        WasmSimulator *sim = wasm_sim_new(1);
        uint8_t prog[1] = {WASM_OP_END};
        WasmStepTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(wasm_sim_run(sim, prog, 1, 10, &traces, &count) == WASM_OK);
        ISO_CHECK_EQ_UINT(count, 1u);
        wasm_traces_free(traces, count);
        WasmStepTrace t;
        ISO_CHECK(wasm_sim_step(sim, &t) == WASM_ERR_HALTED);
        wasm_sim_free(sim);
    }

    /* ── an unknown opcode halts the run with an error (the Rust panic) ──── */
    {
        WasmSimulator *sim = wasm_sim_new(1);
        uint8_t prog[1] = {0xFF};
        WasmStepTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(wasm_sim_run(sim, prog, 1, 10, &traces, &count) ==
                  WASM_ERR_UNKNOWN_OPCODE);
        ISO_CHECK(traces == NULL && count == 0);
        wasm_sim_free(sim);
    }

    /* ── stack underflow on an empty add is reported ────────────────────── */
    {
        WasmSimulator *sim = wasm_sim_new(1);
        uint8_t prog[1] = {WASM_OP_I32_ADD};
        WasmStepTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(wasm_sim_run(sim, prog, 1, 10, &traces, &count) ==
                  WASM_ERR_STACK_UNDERFLOW);
        wasm_sim_free(sim);
    }

    /* ── i32.add wraps modulo 2^32 ──────────────────────────────────────── */
    {
        WasmSimulator *sim = wasm_sim_new(0);
        WasmProgram p;
        wasm_program_init(&p);
        wasm_emit_i32_const(&p, 2147483647); /* INT32_MAX */
        wasm_emit_i32_const(&p, 1);
        wasm_emit_i32_add(&p);
        wasm_emit_end(&p);
        size_t plen;
        const uint8_t *bytes = wasm_program_bytes(&p, &plen);
        WasmStepTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(wasm_sim_run(sim, bytes, plen, 100, &traces, &count) ==
                  WASM_OK);
        size_t nstack;
        const int32_t *stack = wasm_sim_stack(sim, &nstack);
        ISO_CHECK(stack[0] == (-2147483647 - 1)); /* wraps to INT32_MIN */
        wasm_traces_free(traces, count);
        wasm_program_free(&p);
        wasm_sim_free(sim);
    }

    return ISO_TEST_RESULT();
}
