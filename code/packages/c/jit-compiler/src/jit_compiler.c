/*
 * jit_compiler.c — hot-path profiler + shell native-block registry (impl).
 * ===========================================================================
 *
 * A faithful C port of the Rust `jit-compiler` crate. Two maps drive everything:
 *
 *   - execution_counts: bytecode offset -> how many times it has run; and
 *   - native_blocks:    bytecode offset -> the shell block installed for it.
 *
 * The Rust uses `BTreeMap<usize, _>` for both, but the public API only ever does
 * point lookups by offset (get / insert / remove / contains) — it never exposes
 * iteration order — so the *ordering* a BTreeMap gives is not observable. We
 * therefore back each map with a plain growable array and a linear scan, exactly
 * as the `vault-revisions` port does. Same observable behaviour, no tree.
 */
#include "jit_compiler/jit_compiler.h"

#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, memmove, strlen */

/* execution_counts entry: one bytecode offset and its run count. */
typedef struct {
    size_t offset;
    uint64_t count;
} jit_count_entry;

/* native_blocks entry: one bytecode offset and the block installed there. */
typedef struct {
    size_t offset;
    jit_native_block block;
} jit_block_entry;

struct jit_compiler {
    jit_config config;
    jit_count_entry *counts;
    size_t ncounts;
    size_t ccounts; /* capacity */
    jit_block_entry *blocks;
    size_t nblocks;
    size_t cblocks; /* capacity */
};

/* ------------------------------------------------------------------------- *
 * Small helpers
 * ------------------------------------------------------------------------- */

/* imds__strdup-style duplicate (ISO C has no strdup). NULL on OOM. */
static char *jit__strdup(const char *s) {
    size_t len = strlen(s);
    char *dst = (char *)malloc(len + 1);
    if (!dst) {
        return NULL;
    }
    memcpy(dst, s, len + 1);
    return dst;
}

/*
 * jit__assumptions_dup — deep-copy `n` NUL-terminated strings into a fresh array.
 * n == 0 yields NULL (no storage). On any failure unwinds the prefix and returns
 * NULL. Caller has already validated that no element is NULL when n > 0.
 */
static char **jit__assumptions_dup(const char *const *src, size_t n) {
    char **copy;
    size_t i;
    if (n == 0) {
        return NULL;
    }
    copy = (char **)calloc(n, sizeof(*copy));
    if (!copy) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        copy[i] = jit__strdup(src[i]);
        if (!copy[i]) {
            size_t j;
            for (j = 0; j < i; j++) {
                free(copy[j]);
            }
            free(copy);
            return NULL;
        }
    }
    return copy;
}

/* Free a block's owned heap (does not free the block struct itself). */
static void jit__block_free_inner(jit_native_block *block) {
    size_t i;
    free(block->machine_code);
    block->machine_code = NULL;
    for (i = 0; i < block->nassumptions; i++) {
        free(block->assumptions[i]);
    }
    free(block->assumptions);
    block->assumptions = NULL;
    block->nassumptions = 0;
}

/* Linear scan of the counts array; returns the entry or NULL. */
static jit_count_entry *jit__find_count(const jit_compiler *jit, size_t offset) {
    size_t i;
    for (i = 0; i < jit->ncounts; i++) {
        if (jit->counts[i].offset == offset) {
            return &jit->counts[i];
        }
    }
    return NULL;
}

/* Linear scan of the blocks array; returns the entry or NULL. */
static jit_block_entry *jit__find_block(const jit_compiler *jit, size_t offset) {
    size_t i;
    for (i = 0; i < jit->nblocks; i++) {
        if (jit->blocks[i].offset == offset) {
            return &jit->blocks[i];
        }
    }
    return NULL;
}

/* Ensure at least one free slot in a growable array, doubling capacity. Returns
 * the (possibly moved) base pointer, or NULL on OOM (leaving *cap unchanged). */
static void *jit__grow(void *base, size_t used, size_t *cap, size_t elem_size) {
    size_t new_cap;
    void *grown;
    if (used < *cap) {
        return base;
    }
    /* Double the capacity, but compute the guard BEFORE the multiply so the
     * doubling itself cannot wrap: bail if *cap*2*elem_size would overflow. */
    if (*cap == 0) {
        new_cap = 4;
    } else if (*cap > ((size_t)-1) / elem_size / 2) {
        return NULL;
    } else {
        new_cap = *cap * 2;
    }
    grown = realloc(base, new_cap * elem_size);
    if (!grown) {
        return NULL;
    }
    *cap = new_cap;
    return grown;
}

/* ------------------------------------------------------------------------- *
 * Config
 * ------------------------------------------------------------------------- */

jit_status jit_config_new(jit_isa target, uint64_t hot_threshold, jit_config *out) {
    if (!out || hot_threshold == 0) {
        return JIT_ERR_INVALID;
    }
    out->hot_threshold = hot_threshold;
    out->target = target;
    return JIT_OK;
}

/* ------------------------------------------------------------------------- *
 * Native block
 * ------------------------------------------------------------------------- */

void jit_native_block_free(jit_native_block *block) {
    if (!block) {
        return;
    }
    jit__block_free_inner(block);
}

/* ------------------------------------------------------------------------- *
 * Compiler lifecycle
 * ------------------------------------------------------------------------- */

jit_status jit_compiler_create(const jit_config *config, jit_compiler **out) {
    jit_compiler *jit;
    if (!config || !out) {
        return JIT_ERR_INVALID;
    }
    jit = (jit_compiler *)malloc(sizeof(*jit));
    if (!jit) {
        return JIT_ERR_NOMEM;
    }
    jit->config = *config;
    jit->counts = NULL;
    jit->ncounts = 0;
    jit->ccounts = 0;
    jit->blocks = NULL;
    jit->nblocks = 0;
    jit->cblocks = 0;
    *out = jit;
    return JIT_OK;
}

void jit_compiler_destroy(jit_compiler *jit) {
    size_t i;
    if (!jit) {
        return;
    }
    free(jit->counts);
    for (i = 0; i < jit->nblocks; i++) {
        jit__block_free_inner(&jit->blocks[i].block);
    }
    free(jit->blocks);
    free(jit);
}

/* ------------------------------------------------------------------------- *
 * Profiling
 * ------------------------------------------------------------------------- */

jit_status jit_compiler_observe_execution(jit_compiler *jit, size_t bytecode_offset,
                                          int *became_hot) {
    jit_count_entry *entry;
    if (!jit || !became_hot) {
        return JIT_ERR_INVALID;
    }
    entry = jit__find_count(jit, bytecode_offset);
    if (!entry) {
        /* First execution of this offset: append a fresh counter (or_insert(0)). */
        jit_count_entry *grown =
            (jit_count_entry *)jit__grow(jit->counts, jit->ncounts, &jit->ccounts,
                                         sizeof(*jit->counts));
        if (!grown) {
            return JIT_ERR_NOMEM;
        }
        jit->counts = grown;
        entry = &jit->counts[jit->ncounts++];
        entry->offset = bytecode_offset;
        entry->count = 0;
    }
    entry->count += 1;
    /* Rust returns `*count == hot_threshold` — true exactly on the transition. */
    *became_hot = (entry->count == jit->config.hot_threshold) ? 1 : 0;
    return JIT_OK;
}

jit_status jit_compiler_profile(const jit_compiler *jit, size_t bytecode_offset,
                                jit_hot_path_profile *out, int *found) {
    jit_count_entry *entry;
    if (!jit || !out || !found) {
        return JIT_ERR_INVALID;
    }
    entry = jit__find_count(jit, bytecode_offset);
    if (!entry) {
        *found = 0;
        return JIT_OK;
    }
    out->bytecode_offset = bytecode_offset;
    out->execution_count = entry->count;
    out->is_hot = (entry->count >= jit->config.hot_threshold) ? 1 : 0;
    *found = 1;
    return JIT_OK;
}

/* ------------------------------------------------------------------------- *
 * Block registry
 * ------------------------------------------------------------------------- */

jit_status jit_compiler_install_shell_block(jit_compiler *jit, size_t bytecode_offset,
                                            const char *const *assumptions,
                                            size_t nassumptions,
                                            const jit_native_block **out) {
    char **assumptions_copy;
    jit_block_entry *entry;
    size_t i;
    if (!jit || !out || (!assumptions && nassumptions > 0)) {
        return JIT_ERR_INVALID;
    }
    /* Rust's Vec<String> can hold no nulls — reject a NULL element up front so
     * the copy never dereferences one. */
    for (i = 0; i < nassumptions; i++) {
        if (!assumptions[i]) {
            return JIT_ERR_INVALID;
        }
    }
    assumptions_copy = jit__assumptions_dup(assumptions, nassumptions);
    if (nassumptions > 0 && !assumptions_copy) {
        return JIT_ERR_NOMEM;
    }

    entry = jit__find_block(jit, bytecode_offset);
    if (entry) {
        /* Replace: free the old block's heap, keep the slot. (BTreeMap::insert
         * overwrites the value at an existing key.) */
        jit__block_free_inner(&entry->block);
    } else {
        jit_block_entry *grown =
            (jit_block_entry *)jit__grow(jit->blocks, jit->nblocks, &jit->cblocks,
                                         sizeof(*jit->blocks));
        if (!grown) {
            /* Undo the assumptions copy so nothing leaks on the OOM path. */
            for (i = 0; i < nassumptions; i++) {
                free(assumptions_copy[i]);
            }
            free(assumptions_copy);
            return JIT_ERR_NOMEM;
        }
        jit->blocks = grown;
        entry = &jit->blocks[jit->nblocks++];
        entry->offset = bytecode_offset;
    }
    entry->block.bytecode_offset = bytecode_offset;
    entry->block.target = jit->config.target;
    entry->block.machine_code = NULL; /* always empty (Vec::new()) */
    entry->block.machine_code_len = 0;
    entry->block.assumptions = assumptions_copy;
    entry->block.nassumptions = nassumptions;
    *out = &entry->block;
    return JIT_OK;
}

int jit_compiler_has_native_block(const jit_compiler *jit, size_t bytecode_offset) {
    if (!jit) {
        return 0;
    }
    return jit__find_block(jit, bytecode_offset) != NULL ? 1 : 0;
}

const jit_native_block *jit_compiler_native_block(const jit_compiler *jit,
                                                  size_t bytecode_offset) {
    jit_block_entry *entry;
    if (!jit) {
        return NULL;
    }
    entry = jit__find_block(jit, bytecode_offset);
    return entry ? &entry->block : NULL;
}

jit_status jit_compiler_deoptimize(jit_compiler *jit, size_t bytecode_offset,
                                   jit_native_block *out, int *found) {
    jit_block_entry *entry;
    size_t index;
    if (!jit || !out || !found) {
        return JIT_ERR_INVALID;
    }
    entry = jit__find_block(jit, bytecode_offset);
    if (!entry) {
        *found = 0;
        return JIT_OK;
    }
    /* MOVE the block out: copy the struct (transferring its owned pointers) to
     * the caller, then drop the slot WITHOUT freeing those pointers. */
    *out = entry->block;
    index = (size_t)(entry - jit->blocks);
    memmove(&jit->blocks[index], &jit->blocks[index + 1],
            (jit->nblocks - index - 1) * sizeof(*jit->blocks));
    jit->nblocks--;
    *found = 1;
    return JIT_OK;
}

const jit_config *jit_compiler_config(const jit_compiler *jit) {
    if (!jit) {
        return NULL;
    }
    return &jit->config;
}
