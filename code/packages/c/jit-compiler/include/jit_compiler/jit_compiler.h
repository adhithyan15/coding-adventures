/*
 * jit_compiler/jit_compiler.h — hot-path profiler + shell native-block registry.
 * ===========================================================================
 *
 * The C port of the Rust `jit-compiler` crate, and the second bucket-A port of
 * the CCPP02 campaign: a pure-ISO crate that needs no OS, so it rides the
 * `iso-harness` (links nothing, compiled with `-pedantic-errors` / `/permissive-`).
 *
 * WHAT IT IS (AND ISN'T). A real JIT does two jobs: (1) decide which bytecode is
 * worth compiling, and (2) manage the native blocks that replace interpretation.
 * This crate — faithfully to the Rust — implements only the *management* layers
 * and NOT code generation:
 *
 *   - hot-path execution profiling (a per-offset execution counter);
 *   - threshold-based "this path just went hot" detection;
 *   - a registry of *shell* native blocks (the machine-code buffer is always
 *     empty — the shape of a future JIT without the hard part); and
 *   - deoptimization (remove a block, fall back to interpretation).
 *
 * Do not confuse this with os-platform's `jit` primitive, which allocates real
 * executable memory. This package emits nothing and touches no OS; it is a pure
 * bookkeeping layer.
 *
 * OWNERSHIP. A `jit_compiler` owns everything it stores. `install_shell_block`
 * and `native_block` hand back a BORROWED pointer into the registry's backing
 * array. That pointer is invalidated by the VERY NEXT call that mutates the
 * registry, and must not be used afterwards: `install_shell_block` may `realloc`
 * the array (so every outstanding borrow dangles), and `deoptimize` compacts it
 * (so a borrow may silently come to refer to a *different* block or past the
 * end). This is the C cost of representing Rust's `&NativeBlock`, whose borrow
 * checker forbids exactly this aliasing. Copy out what you need before mutating.
 * `deoptimize` MOVES a block out to the caller, who then owns it and must release
 * it with `jit_native_block_free`. `jit_compiler_destroy` frees the rest.
 */
#ifndef JIT_COMPILER_JIT_COMPILER_H
#define JIT_COMPILER_JIT_COMPILER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Every result the profiler/registry can produce. */
typedef enum {
    JIT_OK = 0,
    JIT_ERR_INVALID, /* NULL out, threshold 0, NULL assumption, etc. */
    JIT_ERR_NOMEM    /* allocation failure */
} jit_status;

/* Target architecture a future code generator would emit for. */
typedef enum {
    JIT_ISA_RISCV,
    JIT_ISA_ARM,
    JIT_ISA_X86
} jit_isa;

/*
 * Configuration for a compiler instance. A transparent value type: read the
 * fields directly. Build one with jit_config_new so the `hot_threshold > 0`
 * invariant (the Rust `assert!`) is enforced in one place.
 */
typedef struct {
    uint64_t hot_threshold;
    jit_isa target;
} jit_config;

/*
 * jit_config_new — fill *out with a validated config. JIT_ERR_INVALID if out is
 * NULL or hot_threshold is 0 (the Rust constructor asserts threshold > 0).
 */
jit_status jit_config_new(jit_isa target, uint64_t hot_threshold, jit_config *out);

/* A profiling snapshot for one bytecode offset (a copy — owns nothing). */
typedef struct {
    size_t bytecode_offset;
    uint64_t execution_count;
    int is_hot;
} jit_hot_path_profile;

/*
 * A shell native block. `machine_code` is always empty in this implementation
 * (it exists so the API already has a future JIT's shape). Owns `machine_code`
 * and the `assumptions` string array.
 */
typedef struct {
    size_t bytecode_offset;
    jit_isa target;
    unsigned char *machine_code;
    size_t machine_code_len;
    char **assumptions;
    size_t nassumptions;
} jit_native_block;

/* Release a block obtained from jit_compiler_deoptimize (safe on a zeroed value
 * and on NULL). Do NOT call it on a *borrowed* block from install/native_block —
 * those are owned by the compiler. */
void jit_native_block_free(jit_native_block *block);

/* Opaque threshold-based profiler + block registry. */
typedef struct jit_compiler jit_compiler;

/* Create / destroy. jit_compiler_create copies `config`. JIT_ERR_INVALID on a
 * NULL argument, JIT_ERR_NOMEM on allocation failure. */
jit_status jit_compiler_create(const jit_config *config, jit_compiler **out);
void jit_compiler_destroy(jit_compiler *jit);

/*
 * jit_compiler_observe_execution — record one execution at `bytecode_offset` and
 * report, via *became_hot, whether the path transitions to hot *on this call*
 * (i.e. its count just reached the threshold — true exactly once). JIT_ERR_NOMEM
 * if a first-time offset can't be tracked; JIT_ERR_INVALID on NULL args.
 */
jit_status jit_compiler_observe_execution(jit_compiler *jit, size_t bytecode_offset,
                                          int *became_hot);

/*
 * jit_compiler_profile — the snapshot for one offset. Sets *found to 1 and fills
 * *out when the offset has ever executed, else *found to 0 (the Rust Option).
 * JIT_ERR_INVALID on NULL args.
 */
jit_status jit_compiler_profile(const jit_compiler *jit, size_t bytecode_offset,
                                jit_hot_path_profile *out, int *found);

/*
 * jit_compiler_install_shell_block — register a shell block for `bytecode_offset`
 * (target = the configured ISA, empty machine code, a copy of `assumptions`),
 * replacing any block already at that offset. On success *out points to the
 * stored block — BORROWED, valid only until the next registry-mutating call.
 * `assumptions` is an array of `nassumptions` NUL-terminated strings (NULL/0 for
 * none); JIT_ERR_INVALID if it or any element is NULL when nassumptions > 0.
 */
jit_status jit_compiler_install_shell_block(jit_compiler *jit, size_t bytecode_offset,
                                            const char *const *assumptions,
                                            size_t nassumptions,
                                            const jit_native_block **out);

/* Whether a block is registered for this offset (NULL jit → 0). */
int jit_compiler_has_native_block(const jit_compiler *jit, size_t bytecode_offset);

/* Borrow the registered block for this offset, or NULL. Borrowed — do not free;
 * invalidated by the next registry-mutating call. */
const jit_native_block *jit_compiler_native_block(const jit_compiler *jit,
                                                  size_t bytecode_offset);

/*
 * jit_compiler_deoptimize — remove the block at `bytecode_offset` and MOVE it to
 * the caller. Sets *found to 1 and fills *out (now owned by the caller — release
 * with jit_native_block_free) when a block was present, else *found to 0 (the
 * Rust Option::None). JIT_ERR_INVALID on NULL args.
 */
jit_status jit_compiler_deoptimize(jit_compiler *jit, size_t bytecode_offset,
                                   jit_native_block *out, int *found);

/* Borrow the compiler's configuration (valid for the compiler's lifetime). */
const jit_config *jit_compiler_config(const jit_compiler *jit);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* JIT_COMPILER_JIT_COMPILER_H */
