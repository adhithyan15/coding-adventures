/*
 * Tests for the C jvm-simulator, using the header-only iso_test.h harness (pure
 * ISO). The program vectors mirror the Rust crate's own tests; the error cases
 * replace the Rust panics with status codes.
 */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "jvm_simulator.h"

int main(void) {
    /* ── basic program: x = 1 + 2; return x  (returns 3) ─────────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        ISO_CHECK(s != NULL);
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_ICONST_0 + 1, 0); /* iconst_1 */
        jvm_emit(&p, JVM_OP_ICONST_0 + 2, 0); /* iconst_2 */
        jvm_emit(&p, JVM_OP_IADD, 0);
        jvm_emit(&p, JVM_OP_ISTORE_0, 0);
        jvm_emit(&p, JVM_OP_ILOAD_0, 0);
        jvm_emit(&p, JVM_OP_IRETURN, 0);

        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        ISO_CHECK_EQ_UINT(plen, 6u); /* all single-byte opcodes */
        ISO_CHECK(jvm_sim_load(s, bytes, plen, NULL, 0, 16) == 0);

        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 100, &traces, &count) == JVM_OK);
        ISO_CHECK_EQ_UINT(count, 6u);

        int32_t rv;
        ISO_CHECK(jvm_sim_return_value(s, &rv) == 1);
        ISO_CHECK(rv == 3);
        ISO_CHECK(jvm_sim_halted(s));

        /* A few trace details. */
        ISO_CHECK_STR_EQ(traces[0].opcode, "iconst_1");
        ISO_CHECK_STR_EQ(traces[0].description, "push 1");
        ISO_CHECK(traces[0].n_stack_before == 0 &&
                  traces[0].n_stack_after == 1);
        ISO_CHECK(traces[2].stack_after[0] == 3); /* after iadd */
        ISO_CHECK_STR_EQ(traces[3].opcode, "istore_0");
        ISO_CHECK_STR_EQ(traces[3].description, "pop 3, store in locals[0]");
        ISO_CHECK(traces[3].locals_snapshot[0].initialized &&
                  traces[3].locals_snapshot[0].value == 3);
        ISO_CHECK_STR_EQ(traces[5].description, "return 3");

        jvm_traces_free(traces, count);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── ldc + bipush with a negative value: -42 - 100 = -142 ────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_BIPUSH, -42); /* 1-byte signed operand */
        jvm_emit(&p, JVM_OP_LDC, 0);      /* constant[0] = 100 */
        jvm_emit(&p, JVM_OP_ISUB, 0);
        jvm_emit(&p, JVM_OP_IRETURN, 0);

        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        int32_t constants[1] = {100};
        ISO_CHECK(jvm_sim_load(s, bytes, plen, constants, 1, 16) == 0);

        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 100, &traces, &count) == JVM_OK);
        int32_t rv;
        ISO_CHECK(jvm_sim_return_value(s, &rv) == 1);
        ISO_CHECK(rv == -142);
        ISO_CHECK_STR_EQ(traces[1].description, "push constant[0] = 100");

        jvm_traces_free(traces, count);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── if_icmpeq branch taken: 5 == 5 jumps over iconst_1 to iconst_4 ──── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_ICONST_0 + 5, 0);  /* pc 0: push 5 */
        jvm_emit(&p, JVM_OP_ICONST_0 + 5, 0);  /* pc 1: push 5 */
        jvm_emit(&p, JVM_OP_IF_ICMPEQ, 4);     /* pc 2: +4 -> pc 6 */
        jvm_emit(&p, JVM_OP_ICONST_0 + 1, 0);  /* pc 5: skipped */
        jvm_emit(&p, JVM_OP_ICONST_0 + 4, 0);  /* pc 6: push 4 (target) */
        jvm_emit(&p, JVM_OP_IRETURN, 0);       /* pc 7 */

        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        ISO_CHECK(jvm_sim_load(s, bytes, plen, NULL, 0, 16) == 0);

        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 10, &traces, &count) == JVM_OK);
        int32_t rv;
        ISO_CHECK(jvm_sim_return_value(s, &rv) == 1);
        ISO_CHECK(rv == 4);

        int found = 0;
        for (size_t i = 0; i < count; i++)
            if (strcmp(traces[i].opcode, "if_icmpeq") == 0) found = 1;
        ISO_CHECK(found);

        jvm_traces_free(traces, count);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── division by zero is reported (the Rust panic) ──────────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_ICONST_0 + 5, 0);
        jvm_emit(&p, JVM_OP_ICONST_0, 0); /* push 0 */
        jvm_emit(&p, JVM_OP_IDIV, 0);

        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        ISO_CHECK(jvm_sim_load(s, bytes, plen, NULL, 0, 16) == 0);
        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 10, &traces, &count) == JVM_ERR_DIV_BY_ZERO);
        ISO_CHECK(traces == NULL && count == 0);

        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── idiv on INT32_MIN / -1 wraps to INT32_MIN (no C UB) ─────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_LDC, 0); /* INT32_MIN */
        jvm_emit(&p, JVM_OP_LDC, 1); /* -1 */
        jvm_emit(&p, JVM_OP_IDIV, 0);
        jvm_emit(&p, JVM_OP_IRETURN, 0);

        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        int32_t constants[2] = {INT32_MIN, -1};
        ISO_CHECK(jvm_sim_load(s, bytes, plen, constants, 2, 16) == 0);
        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 10, &traces, &count) == JVM_OK);
        int32_t rv;
        ISO_CHECK(jvm_sim_return_value(s, &rv) == 1);
        ISO_CHECK(rv == INT32_MIN);
        jvm_traces_free(traces, count);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── stepping a halted VM is reported (the Rust panic) ──────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_RETURN, 0);
        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        ISO_CHECK(jvm_sim_load(s, bytes, plen, NULL, 0, 16) == 0);
        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 10, &traces, &count) == JVM_OK);
        ISO_CHECK_EQ_UINT(count, 1u);
        jvm_traces_free(traces, count);
        JvmTrace t;
        ISO_CHECK(jvm_sim_step(s, &t) == JVM_ERR_HALTED);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── an uninitialized local load is reported ────────────────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_ILOAD_0, 0); /* slot 0 never stored */
        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        ISO_CHECK(jvm_sim_load(s, bytes, plen, NULL, 0, 16) == 0);
        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 10, &traces, &count) ==
                  JVM_ERR_LOCAL_UNINITIALIZED);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    /* ── iadd wraps modulo 2^32 ─────────────────────────────────────────── */
    {
        JvmSimulator *s = jvm_sim_new();
        JvmProgram p;
        jvm_program_init(&p);
        jvm_emit(&p, JVM_OP_LDC, 0); /* INT32_MAX */
        jvm_emit(&p, JVM_OP_ICONST_0 + 1, 0);
        jvm_emit(&p, JVM_OP_IADD, 0);
        jvm_emit(&p, JVM_OP_IRETURN, 0);
        size_t plen;
        const uint8_t *bytes = jvm_program_bytes(&p, &plen);
        int32_t constants[1] = {INT32_MAX};
        ISO_CHECK(jvm_sim_load(s, bytes, plen, constants, 1, 16) == 0);
        JvmTrace *traces = NULL;
        size_t count = 0;
        ISO_CHECK(jvm_sim_run(s, 10, &traces, &count) == JVM_OK);
        int32_t rv;
        ISO_CHECK(jvm_sim_return_value(s, &rv) == 1);
        ISO_CHECK(rv == INT32_MIN); /* MAX + 1 wraps */
        jvm_traces_free(traces, count);
        jvm_program_free(&p);
        jvm_sim_free(s);
    }

    return ISO_TEST_RESULT();
}
