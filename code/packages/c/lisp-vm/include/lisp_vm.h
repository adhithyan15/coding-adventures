/*
 * lisp_vm.h — execute compiled Lisp bytecode, pure ISO C17.
 * =========================================================
 *
 * A faithful port of the Rust `lisp-vm` crate — the last stage of the Lisp
 * toolchain (lexer → parser → compiler → VM). It executes the bytecode
 * (`LcCodeObject`) the compiler produces: a value stack, a global variable
 * table, local slots, a grow-only heap (cons cells, interned symbols,
 * closures), closures with captured environments, and tail-call optimisation.
 *
 * Values are `LcValue`s (from lisp-compiler); the VM clones them onto the stack
 * / into variables / into the heap and frees them as they are consumed, so the
 * whole thing is malloc-owned. `lv_run` runs the full pipeline from source.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef LISP_VM_H
#define LISP_VM_H

#include <stddef.h> /* size_t */

#include "lisp_compiler.h" /* LcValue, LcCodeObject, lc_compile */

#ifdef __cplusplus
extern "C" {
#endif

/* A VM runtime error. */
typedef struct {
    char message[128];
} LvError;

/* Heap object kinds. */
typedef enum { LV_CONS, LV_SYMBOL, LV_CLOSURE } LvHeapKind;

typedef struct LvHeapObject LvHeapObject;
typedef struct LispVm LispVm;

/* ── The VM ────────────────────────────────────────────────────────────────*/

LispVm *lv_new(void);
void lv_free(LispVm *vm);

/* Execute `code` from the start until HALT or the end. Returns 1 on success, 0
 * on a runtime error (fills `*err`). */
int lv_execute(LispVm *vm, const LcCodeObject *code, LvError *err);

/* Stack inspection (borrowed values). */
size_t lv_stack_len(const LispVm *vm);
const LcValue *lv_stack_at(const LispVm *vm, size_t i);
const LcValue *lv_stack_top(const LispVm *vm); /* NULL if empty */

/* Heap inspection (borrowed). */
size_t lv_heap_len(const LispVm *vm);
const LvHeapObject *lv_heap_at(const LispVm *vm, size_t addr);
LvHeapKind lv_heap_kind(const LvHeapObject *o);
const LcValue *lv_cons_car(const LvHeapObject *o); /* LV_CONS */
const LcValue *lv_cons_cdr(const LvHeapObject *o); /* LV_CONS */
const char *lv_symbol_name(const LvHeapObject *o); /* LV_SYMBOL */

/* Output collected from (print ...). */
size_t lv_output_len(const LispVm *vm);
const char *lv_output_at(const LispVm *vm, size_t i);

/* Format a value the way `print` does (owned string; caller frees). */
char *lv_format_value(const LispVm *vm, const LcValue *v);

/* ── Top-level pipeline ────────────────────────────────────────────────────*/

/* Compile and execute `source`, writing the result value (top of stack, or Nil)
 * to `*out` (owned; release with lc_value_free). Returns 1 on success, 0 on a
 * compile or runtime error (fills `*err`). */
int lv_run(const char *source, LcValue *out, LvError *err);

/* Like lv_run but also returns the (print ...) output as an owned array of
 * owned strings (`*out_lines` / `*n_lines`; free each line then the array).
 * On failure `*out_lines` is NULL and `*n_lines` is 0. */
int lv_run_with_output(const char *source, LcValue *out, char ***out_lines,
                       size_t *n_lines, LvError *err);

#ifdef __cplusplus
}
#endif

#endif /* LISP_VM_H */
