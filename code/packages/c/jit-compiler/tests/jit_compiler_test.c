/*
 * jit_compiler_test.c — tests for the hot-path profiler + shell block registry.
 * ===========================================================================
 *
 * Mirrors the four Rust unit tests (threshold transition, profile snapshot,
 * shell-block install, deoptimize) and pins the C ownership contract: a
 * deoptimized block is owned by the caller and frees cleanly; borrowed blocks
 * are not double-freed; the invalid/argument paths are total. Runs under
 * ASan+UBSan so any leak or misuse fails the build.
 */
#include "jit_compiler/jit_compiler.h"
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

/* Build a compiler with the given ISA + threshold; asserts creation succeeds. */
static jit_compiler *make_jit(jit_isa isa, uint64_t threshold) {
    jit_config cfg;
    jit_compiler *jit = NULL;
    ISO_CHECK_EQ_INT(jit_config_new(isa, threshold, &cfg), JIT_OK);
    ISO_CHECK_EQ_INT(jit_compiler_create(&cfg, &jit), JIT_OK);
    ISO_CHECK(jit != NULL);
    return jit;
}

/* Rust: path_becomes_hot_exactly_at_threshold. */
static void test_hot_exactly_at_threshold(void) {
    jit_compiler *jit = make_jit(JIT_ISA_RISCV, 3);
    int hot = -1;
    ISO_CHECK_EQ_INT(jit_compiler_observe_execution(jit, 24, &hot), JIT_OK);
    ISO_CHECK_EQ_INT(hot, 0); /* count 1 */
    ISO_CHECK_EQ_INT(jit_compiler_observe_execution(jit, 24, &hot), JIT_OK);
    ISO_CHECK_EQ_INT(hot, 0); /* count 2 */
    ISO_CHECK_EQ_INT(jit_compiler_observe_execution(jit, 24, &hot), JIT_OK);
    ISO_CHECK_EQ_INT(hot, 1); /* count 3 == threshold → transitions hot */
    ISO_CHECK_EQ_INT(jit_compiler_observe_execution(jit, 24, &hot), JIT_OK);
    ISO_CHECK_EQ_INT(hot, 0); /* count 4 → already hot, not a transition */
    jit_compiler_destroy(jit);
}

/* Rust: profile_reports_execution_count_and_hotness. */
static void test_profile_count_and_hotness(void) {
    jit_compiler *jit = make_jit(JIT_ISA_ARM, 2);
    jit_hot_path_profile p;
    int found = -1, hot = -1;

    /* Unobserved offset → not found. */
    ISO_CHECK_EQ_INT(jit_compiler_profile(jit, 8, &p, &found), JIT_OK);
    ISO_CHECK_EQ_INT(found, 0);

    jit_compiler_observe_execution(jit, 8, &hot);
    ISO_CHECK_EQ_INT(jit_compiler_profile(jit, 8, &p, &found), JIT_OK);
    ISO_CHECK_EQ_INT(found, 1);
    ISO_CHECK_EQ_UINT(p.bytecode_offset, 8);
    ISO_CHECK(p.execution_count == 1);
    ISO_CHECK_EQ_INT(p.is_hot, 0);

    jit_compiler_observe_execution(jit, 8, &hot);
    ISO_CHECK_EQ_INT(jit_compiler_profile(jit, 8, &p, &found), JIT_OK);
    ISO_CHECK(p.execution_count == 2);
    ISO_CHECK_EQ_INT(p.is_hot, 1); /* count 2 >= threshold 2 */
    jit_compiler_destroy(jit);
}

/* Rust: shell_block_installation_uses_configured_target. */
static void test_install_shell_block(void) {
    jit_compiler *jit = make_jit(JIT_ISA_X86, 5);
    const char *assumptions[1];
    const jit_native_block *block = NULL;
    assumptions[0] = "locals stay integers";

    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 32, assumptions, 1, &block),
                     JIT_OK);
    ISO_CHECK(block != NULL);
    ISO_CHECK_EQ_UINT(block->bytecode_offset, 32);
    ISO_CHECK_EQ_INT(block->target, JIT_ISA_X86); /* configured ISA */
    ISO_CHECK(block->machine_code == NULL);        /* empty machine code */
    ISO_CHECK_EQ_UINT(block->machine_code_len, 0);
    ISO_CHECK_EQ_UINT(block->nassumptions, 1);
    ISO_CHECK_STR_EQ(block->assumptions[0], "locals stay integers");
    ISO_CHECK_EQ_INT(jit_compiler_has_native_block(jit, 32), 1);
    ISO_CHECK_EQ_INT(jit_compiler_has_native_block(jit, 99), 0);

    /* native_block() borrows the same stored block. */
    ISO_CHECK(jit_compiler_native_block(jit, 32) != NULL);
    ISO_CHECK(jit_compiler_native_block(jit, 99) == NULL);
    jit_compiler_destroy(jit); /* frees the stored block + its assumption copy */
}

/* Rust: deoptimize_removes_native_block, plus the owned-move contract. */
static void test_deoptimize_moves_block_out(void) {
    jit_compiler *jit = make_jit(JIT_ISA_RISCV, 10);
    const char *assumptions[1];
    const jit_native_block *installed = NULL;
    jit_native_block moved;
    int found = -1;
    assumptions[0] = "shape stays stable";

    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 99, assumptions, 1, &installed),
                     JIT_OK);

    ISO_CHECK_EQ_INT(jit_compiler_deoptimize(jit, 99, &moved, &found), JIT_OK);
    ISO_CHECK_EQ_INT(found, 1);
    ISO_CHECK_EQ_UINT(moved.bytecode_offset, 99);
    ISO_CHECK_EQ_UINT(moved.nassumptions, 1);
    ISO_CHECK_STR_EQ(moved.assumptions[0], "shape stays stable");
    ISO_CHECK_EQ_INT(jit_compiler_has_native_block(jit, 99), 0); /* gone from store */

    /* Second deoptimize of the same offset → None. */
    ISO_CHECK_EQ_INT(jit_compiler_deoptimize(jit, 99, &moved, &found), JIT_OK);
    ISO_CHECK_EQ_INT(found, 0);

    /* Caller now owns the moved block — must free it (store must NOT double-free
     * it at destroy; ASan catches a double-free). */
    jit_native_block_free(&moved);
    jit_compiler_destroy(jit);
}

/* Install replaces the block at an existing offset (no leak of the old copy). */
static void test_install_replaces_existing(void) {
    jit_compiler *jit = make_jit(JIT_ISA_ARM, 4);
    const char *first[1];
    const char *second[2];
    const jit_native_block *block = NULL;
    first[0] = "v1";
    second[0] = "v2a";
    second[1] = "v2b";

    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 7, first, 1, &block), JIT_OK);
    ISO_CHECK_EQ_UINT(block->nassumptions, 1);
    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 7, second, 2, &block), JIT_OK);
    ISO_CHECK_EQ_UINT(block->nassumptions, 2); /* replaced, old "v1" freed */
    ISO_CHECK_STR_EQ(block->assumptions[1], "v2b");
    jit_compiler_destroy(jit);
}

/* Install with no assumptions is valid (empty block). */
static void test_install_no_assumptions(void) {
    jit_compiler *jit = make_jit(JIT_ISA_X86, 1);
    const jit_native_block *block = NULL;
    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 0, NULL, 0, &block), JIT_OK);
    ISO_CHECK(block != NULL);
    ISO_CHECK_EQ_UINT(block->nassumptions, 0);
    ISO_CHECK(block->assumptions == NULL);
    jit_compiler_destroy(jit);
}

/* Distinct offsets grow the registry array independently. */
static void test_many_offsets(void) {
    jit_compiler *jit = make_jit(JIT_ISA_RISCV, 100);
    int hot = -1;
    size_t i;
    for (i = 0; i < 50; i++) {
        ISO_CHECK_EQ_INT(jit_compiler_observe_execution(jit, i * 4, &hot), JIT_OK);
        ISO_CHECK_EQ_INT(hot, 0);
    }
    /* Each offset counted exactly once. */
    for (i = 0; i < 50; i++) {
        jit_hot_path_profile p;
        int found = -1;
        ISO_CHECK_EQ_INT(jit_compiler_profile(jit, i * 4, &p, &found), JIT_OK);
        ISO_CHECK_EQ_INT(found, 1);
        ISO_CHECK(p.execution_count == 1);
    }
    jit_compiler_destroy(jit);
}

static void test_config_accessors(void) {
    jit_compiler *jit = make_jit(JIT_ISA_ARM, 42);
    const jit_config *cfg = jit_compiler_config(jit);
    ISO_CHECK(cfg != NULL);
    ISO_CHECK(cfg->hot_threshold == 42);
    ISO_CHECK_EQ_INT(cfg->target, JIT_ISA_ARM);
    jit_compiler_destroy(jit);
}

static void test_invalid_params(void) {
    jit_config cfg;
    jit_compiler *jit;
    const jit_native_block *block;
    jit_native_block moved;
    jit_hot_path_profile p;
    int flag;
    const char *bad[1];
    bad[0] = NULL;

    /* config: threshold 0 → invalid (Rust asserts > 0); NULL out → invalid. */
    ISO_CHECK_EQ_INT(jit_config_new(JIT_ISA_ARM, 0, &cfg), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_config_new(JIT_ISA_ARM, 1, NULL), JIT_ERR_INVALID);

    ISO_CHECK_EQ_INT(jit_config_new(JIT_ISA_ARM, 1, &cfg), JIT_OK);
    ISO_CHECK_EQ_INT(jit_compiler_create(NULL, &jit), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_create(&cfg, NULL), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_create(&cfg, &jit), JIT_OK);

    ISO_CHECK_EQ_INT(jit_compiler_observe_execution(NULL, 0, &flag), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_observe_execution(jit, 0, NULL), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_profile(jit, 0, NULL, &flag), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_profile(jit, 0, &p, NULL), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_deoptimize(jit, 0, NULL, &flag), JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_deoptimize(jit, 0, &moved, NULL), JIT_ERR_INVALID);
    /* NULL assumptions with n>0, and a NULL element, both invalid. */
    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 0, NULL, 2, &block),
                     JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 0, bad, 1, &block),
                     JIT_ERR_INVALID);
    ISO_CHECK_EQ_INT(jit_compiler_install_shell_block(jit, 0, NULL, 0, NULL),
                     JIT_ERR_INVALID);

    /* NULL-tolerant accessors / frees. */
    ISO_CHECK_EQ_INT(jit_compiler_has_native_block(NULL, 0), 0);
    ISO_CHECK(jit_compiler_native_block(NULL, 0) == NULL);
    ISO_CHECK(jit_compiler_config(NULL) == NULL);
    jit_native_block_free(NULL);
    jit_compiler_destroy(NULL);
    jit_compiler_destroy(jit);
}

int main(void) {
    test_hot_exactly_at_threshold();
    test_profile_count_and_hotness();
    test_install_shell_block();
    test_deoptimize_moves_block_out();
    test_install_replaces_existing();
    test_install_no_assumptions();
    test_many_offsets();
    test_config_accessors();
    test_invalid_params();
    return ISO_TEST_RESULT();
}
