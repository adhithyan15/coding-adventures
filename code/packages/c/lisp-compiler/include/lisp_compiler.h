/*
 * lisp_compiler.h — compile S-expression ASTs into Lisp bytecode, pure ISO C17.
 * ============================================================================
 *
 * A faithful port of the Rust `lisp-compiler` crate. It sits on top of the
 * sibling `lisp-parser` (and `lisp-lexer`): Lisp source in, a `LcCodeObject` of
 * bytecode out. The compiler inspects the first element of each list to assign
 * meaning — `define`, `lambda`, `cond`, `quote`, arithmetic, comparison, cons
 * cells, predicates, or an ordinary function call — and tracks tail position so
 * calls in tail position emit `LC_TAIL_CALL` (tail-call optimisation).
 *
 * The produced `LcCodeObject` (instructions + a constant pool + a name pool) is
 * exposed as a plain struct so a VM — or a test — can walk it directly. Nested
 * lambda bodies are themselves `LcCodeObject`s stored as `LC_VAL_CODE` constants.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no compiler extensions.
 */
#ifndef LISP_COMPILER_H
#define LISP_COMPILER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int64_t */

#include "lisp_parser.h" /* LpProgram, lp_parse */

#ifdef __cplusplus
extern "C" {
#endif

/* Lisp bytecode opcodes. Each is one byte; the high nibble groups by category.*/
typedef enum {
    LC_LOAD_CONST = 0x01,
    LC_POP = 0x02,
    LC_LOAD_NIL = 0x03,
    LC_LOAD_TRUE = 0x04,
    LC_STORE_NAME = 0x10,
    LC_LOAD_NAME = 0x11,
    LC_STORE_LOCAL = 0x12,
    LC_LOAD_LOCAL = 0x13,
    LC_ADD = 0x20,
    LC_SUB = 0x21,
    LC_MUL = 0x22,
    LC_DIV = 0x23,
    LC_CMP_EQ = 0x30,
    LC_CMP_LT = 0x31,
    LC_CMP_GT = 0x32,
    LC_JUMP = 0x40,
    LC_JUMP_IF_FALSE = 0x41,
    LC_JUMP_IF_TRUE = 0x42,
    LC_MAKE_CLOSURE = 0x50,
    LC_CALL_FUNCTION = 0x51,
    LC_TAIL_CALL = 0x52,
    LC_RETURN = 0x53,
    LC_CONS = 0x70,
    LC_CAR = 0x71,
    LC_CDR = 0x72,
    LC_MAKE_SYMBOL = 0x73,
    LC_IS_ATOM = 0x74,
    LC_IS_NIL = 0x75,
    LC_PRINT = 0xA0,
    LC_HALT = 0xFF
} LcOp;

/* A single instruction: an opcode and an optional operand (constant/name index,
 * jump target, or argument count). */
typedef struct {
    LcOp opcode;
    int has_operand;
    size_t operand;
} LcInstruction;

/* The kind of a runtime value. */
typedef enum {
    LC_VAL_INTEGER,
    LC_VAL_STRING,
    LC_VAL_BOOL,
    LC_VAL_NIL,
    LC_VAL_SYMBOL,
    LC_VAL_CONS_ADDR,
    LC_VAL_CLOSURE_ADDR,
    LC_VAL_CODE
} LcValueKind;

struct LcCodeObject; /* forward */

/* A runtime value: on the stack, in the constant pool, or in a variable. */
typedef struct LcValue {
    LcValueKind kind;
    int64_t integer;           /* LC_VAL_INTEGER */
    char *str;                 /* LC_VAL_STRING / LC_VAL_SYMBOL (owned) */
    int boolean;               /* LC_VAL_BOOL */
    size_t addr;               /* LC_VAL_CONS_ADDR / LC_VAL_CLOSURE_ADDR */
    struct LcCodeObject *code; /* LC_VAL_CODE (owned) */
} LcValue;

/* A compiled unit of Lisp code. */
typedef struct LcCodeObject {
    LcInstruction *instructions;
    size_t n_instructions;
    LcValue *constants;
    size_t n_constants;
    char **names;
    size_t n_names;
} LcCodeObject;

/* A compilation (or parse) error. */
typedef struct {
    char message[128];
} LcCompileError;

/* Compile Lisp `source` into `*out` (release with lc_code_object_free). Returns
 * 1 on success, 0 on a parse or compile error (fills `*err`). */
int lc_compile(const char *source, LcCodeObject *out, LcCompileError *err);
/* Compile a pre-parsed program (borrowed). */
int lc_compile_ast(const LpProgram *program, LcCodeObject *out,
                   LcCompileError *err);

/* Release the CONTENTS a code object owns (instructions, the constant pool
 * including nested LC_VAL_CODE bodies, and the name pool). The `code` struct
 * itself is caller-owned (e.g. a stack variable filled by lc_compile) and is
 * not freed. NULL-safe. */
void lc_code_object_free(LcCodeObject *code);

/* Is `v` falsy in Lisp (Nil, Bool(false), or Integer(0))? */
int lc_value_is_falsy(const LcValue *v);

/* Deep-copy a value (dups its string and, for LC_VAL_CODE, its whole nested
 * code object). Useful to a VM that moves values around. */
LcValue lc_value_clone(const LcValue *v);
/* Free the heap a value owns (its string and/or nested code object). Leaves the
 * struct itself (which lives in an array or on the stack) untouched. */
void lc_value_free(LcValue *v);
/* Structural equality of two values (deep for LC_VAL_CODE). */
int lc_value_equal(const LcValue *a, const LcValue *b);
/* Deep-copy a code object (returns a fresh heap object; NULL on OOM). */
LcCodeObject *lc_code_object_clone(const LcCodeObject *code);

#ifdef __cplusplus
}
#endif

#endif /* LISP_COMPILER_H */
