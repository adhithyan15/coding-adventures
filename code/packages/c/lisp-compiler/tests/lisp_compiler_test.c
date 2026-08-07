/*
 * Tests for lisp-compiler, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests: they compile source and inspect
 * the resulting opcodes / constant pool / name pool.
 */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "lisp_compiler.h"

static int has_op(const LcCodeObject *code, LcOp op) {
    for (size_t i = 0; i < code->n_instructions; i++)
        if (code->instructions[i].opcode == op) return 1;
    return 0;
}
static size_t count_op(const LcCodeObject *code, LcOp op) {
    size_t n = 0;
    for (size_t i = 0; i < code->n_instructions; i++)
        if (code->instructions[i].opcode == op) n++;
    return n;
}
static int const_has_int(const LcCodeObject *code, int64_t v) {
    for (size_t i = 0; i < code->n_constants; i++)
        if (code->constants[i].kind == LC_VAL_INTEGER &&
            code->constants[i].integer == v)
            return 1;
    return 0;
}
static int const_has_str(const LcCodeObject *code, const char *s) {
    for (size_t i = 0; i < code->n_constants; i++)
        if (code->constants[i].kind == LC_VAL_STRING &&
            strcmp(code->constants[i].str, s) == 0)
            return 1;
    return 0;
}
static int name_has(const LcCodeObject *code, const char *s) {
    for (size_t i = 0; i < code->n_names; i++)
        if (strcmp(code->names[i], s) == 0) return 1;
    return 0;
}
static size_t count_code_const(const LcCodeObject *code) {
    size_t n = 0;
    for (size_t i = 0; i < code->n_constants; i++)
        if (code->constants[i].kind == LC_VAL_CODE) n++;
    return n;
}
static const LcCodeObject *first_code_const(const LcCodeObject *code) {
    for (size_t i = 0; i < code->n_constants; i++)
        if (code->constants[i].kind == LC_VAL_CODE)
            return code->constants[i].code;
    return NULL;
}
/* Operand of the first instruction with opcode `op`; SIZE_MAX if none. */
static size_t op_operand(const LcCodeObject *code, LcOp op) {
    for (size_t i = 0; i < code->n_instructions; i++)
        if (code->instructions[i].opcode == op)
            return code->instructions[i].operand;
    return (size_t)-1;
}

/* Compile `src`; abort the check if it errors. */
#define COMPILE(dst, src)                                     \
    LcCodeObject dst;                                         \
    do {                                                      \
        LcCompileError err_;                                  \
        ISO_CHECK(lc_compile((src), &(dst), &err_));          \
    } while (0)

int main(void) {
    /* ── atoms ───────────────────────────────────────────────────────────────*/
    { COMPILE(c, "42"); ISO_CHECK(const_has_int(&c, 42) && has_op(&c, LC_LOAD_CONST)); lc_code_object_free(&c); }
    { COMPILE(c, "-7"); ISO_CHECK(const_has_int(&c, -7)); lc_code_object_free(&c); }
    { COMPILE(c, "nil"); ISO_CHECK(has_op(&c, LC_LOAD_NIL)); lc_code_object_free(&c); }
    { COMPILE(c, "t"); ISO_CHECK(has_op(&c, LC_LOAD_TRUE)); lc_code_object_free(&c); }
    { COMPILE(c, "x"); ISO_CHECK(name_has(&c, "x") && has_op(&c, LC_LOAD_NAME)); lc_code_object_free(&c); }
    { COMPILE(c, "\"hello\""); ISO_CHECK(const_has_str(&c, "hello")); lc_code_object_free(&c); }

    /* ── arithmetic ──────────────────────────────────────────────────────────*/
    { COMPILE(c, "(+ 1 2)"); ISO_CHECK(has_op(&c, LC_LOAD_CONST) && has_op(&c, LC_ADD)); lc_code_object_free(&c); }
    { COMPILE(c, "(- 5 3)"); ISO_CHECK(has_op(&c, LC_SUB)); lc_code_object_free(&c); }
    { COMPILE(c, "(* 4 5)"); ISO_CHECK(has_op(&c, LC_MUL)); lc_code_object_free(&c); }
    { COMPILE(c, "(/ 10 2)"); ISO_CHECK(has_op(&c, LC_DIV)); lc_code_object_free(&c); }
    { COMPILE(c, "(+ (* 2 3) 4)"); ISO_CHECK(has_op(&c, LC_MUL) && has_op(&c, LC_ADD)); lc_code_object_free(&c); }
    { COMPILE(c, "(+ 1 2)"); ISO_CHECK(const_has_int(&c, 1) && const_has_int(&c, 2)); lc_code_object_free(&c); }

    /* ── comparison ──────────────────────────────────────────────────────────*/
    { COMPILE(c, "(eq 1 2)"); ISO_CHECK(has_op(&c, LC_CMP_EQ)); lc_code_object_free(&c); }
    { COMPILE(c, "(< 1 2)"); ISO_CHECK(has_op(&c, LC_CMP_LT)); lc_code_object_free(&c); }
    { COMPILE(c, "(> 3 2)"); ISO_CHECK(has_op(&c, LC_CMP_GT)); lc_code_object_free(&c); }
    { COMPILE(c, "(= 1 1)"); ISO_CHECK(has_op(&c, LC_CMP_EQ)); lc_code_object_free(&c); }

    /* ── define ──────────────────────────────────────────────────────────────*/
    { COMPILE(c, "(define x 42)"); ISO_CHECK(name_has(&c, "x") && const_has_int(&c, 42) && has_op(&c, LC_STORE_NAME)); lc_code_object_free(&c); }
    { COMPILE(c, "(define x 42)"); ISO_CHECK(has_op(&c, LC_LOAD_NIL)); lc_code_object_free(&c); }

    /* ── cons / car / cdr ────────────────────────────────────────────────────*/
    { COMPILE(c, "(cons 1 2)"); ISO_CHECK(has_op(&c, LC_CONS)); lc_code_object_free(&c); }
    { COMPILE(c, "(car x)"); ISO_CHECK(has_op(&c, LC_CAR)); lc_code_object_free(&c); }
    { COMPILE(c, "(cdr x)"); ISO_CHECK(has_op(&c, LC_CDR)); lc_code_object_free(&c); }

    /* ── predicates ──────────────────────────────────────────────────────────*/
    { COMPILE(c, "(atom x)"); ISO_CHECK(has_op(&c, LC_IS_ATOM)); lc_code_object_free(&c); }
    { COMPILE(c, "(is-nil x)"); ISO_CHECK(has_op(&c, LC_IS_NIL)); lc_code_object_free(&c); }

    /* ── quote ───────────────────────────────────────────────────────────────*/
    { COMPILE(c, "(quote 42)"); ISO_CHECK(has_op(&c, LC_LOAD_CONST)); lc_code_object_free(&c); }
    { COMPILE(c, "(quote foo)"); ISO_CHECK(const_has_str(&c, "foo") && has_op(&c, LC_MAKE_SYMBOL)); lc_code_object_free(&c); }
    { COMPILE(c, "(quote nil)"); ISO_CHECK(has_op(&c, LC_LOAD_NIL)); lc_code_object_free(&c); }
    { COMPILE(c, "(quote (1 2 3))"); ISO_CHECK(has_op(&c, LC_LOAD_NIL) && count_op(&c, LC_CONS) == 3); lc_code_object_free(&c); }
    { COMPILE(c, "(quote ())"); ISO_CHECK(has_op(&c, LC_LOAD_NIL)); lc_code_object_free(&c); }
    { COMPILE(c, "'foo"); ISO_CHECK(has_op(&c, LC_MAKE_SYMBOL)); lc_code_object_free(&c); }
    { COMPILE(c, "'(1 2)"); ISO_CHECK(count_op(&c, LC_CONS) == 2); lc_code_object_free(&c); }

    /* ── cond ────────────────────────────────────────────────────────────────*/
    { COMPILE(c, "(cond ((eq 1 1) 42) (t 0))"); ISO_CHECK(has_op(&c, LC_JUMP_IF_FALSE) && has_op(&c, LC_JUMP)); lc_code_object_free(&c); }
    { COMPILE(c, "(cond (t 42))"); ISO_CHECK(has_op(&c, LC_LOAD_CONST)); lc_code_object_free(&c); }

    /* ── lambda ──────────────────────────────────────────────────────────────*/
    { COMPILE(c, "(lambda (x) x)"); ISO_CHECK(has_op(&c, LC_LOAD_CONST) && has_op(&c, LC_MAKE_CLOSURE)); lc_code_object_free(&c); }
    { COMPILE(c, "(lambda (x) x)"); ISO_CHECK(count_code_const(&c) == 1); lc_code_object_free(&c); }
    { COMPILE(c, "(lambda (x) x)"); const LcCodeObject *b = first_code_const(&c); ISO_CHECK(b != NULL && has_op(b, LC_LOAD_LOCAL)); lc_code_object_free(&c); }
    { COMPILE(c, "(lambda (x) x)"); const LcCodeObject *b = first_code_const(&c); ISO_CHECK(b != NULL && has_op(b, LC_RETURN)); lc_code_object_free(&c); }
    { COMPILE(c, "(lambda (a b c) a)"); ISO_CHECK(count_op(&c, LC_MAKE_CLOSURE) == 1 && op_operand(&c, LC_MAKE_CLOSURE) == 3); lc_code_object_free(&c); }

    /* ── function calls ──────────────────────────────────────────────────────*/
    { COMPILE(c, "(f 1 2)"); ISO_CHECK(has_op(&c, LC_CALL_FUNCTION)); lc_code_object_free(&c); }
    { COMPILE(c, "(f 1 2 3)"); ISO_CHECK(count_op(&c, LC_CALL_FUNCTION) == 1 && op_operand(&c, LC_CALL_FUNCTION) == 3); lc_code_object_free(&c); }

    /* ── tail calls ──────────────────────────────────────────────────────────*/
    { COMPILE(c, "(lambda (n) (f n))"); const LcCodeObject *b = first_code_const(&c); ISO_CHECK(b != NULL && has_op(b, LC_TAIL_CALL) && !has_op(b, LC_CALL_FUNCTION)); lc_code_object_free(&c); }
    { COMPILE(c, "(f 1)"); ISO_CHECK(has_op(&c, LC_CALL_FUNCTION) && !has_op(&c, LC_TAIL_CALL)); lc_code_object_free(&c); }
    { COMPILE(c, "(lambda (n) (g (f n)))"); const LcCodeObject *b = first_code_const(&c); ISO_CHECK(b != NULL && has_op(b, LC_TAIL_CALL) && has_op(b, LC_CALL_FUNCTION)); lc_code_object_free(&c); }
    { COMPILE(c, "(lambda (n) (cond ((eq n 0) 1) (t (f n))))"); const LcCodeObject *b = first_code_const(&c); ISO_CHECK(b != NULL && has_op(b, LC_TAIL_CALL)); lc_code_object_free(&c); }

    /* ── programs ────────────────────────────────────────────────────────────*/
    { COMPILE(c, "1 2 3"); ISO_CHECK(count_op(&c, LC_POP) == 2); lc_code_object_free(&c); }
    { COMPILE(c, "(define x 5) x"); ISO_CHECK(name_has(&c, "x") && has_op(&c, LC_STORE_NAME) && has_op(&c, LC_LOAD_NAME)); lc_code_object_free(&c); }
    { COMPILE(c, ""); ISO_CHECK(c.n_instructions == 1 && c.instructions[0].opcode == LC_HALT); lc_code_object_free(&c); }
    { COMPILE(c, "()"); ISO_CHECK(has_op(&c, LC_LOAD_NIL)); lc_code_object_free(&c); }

    /* ── print ───────────────────────────────────────────────────────────────*/
    { COMPILE(c, "(print 42)"); ISO_CHECK(has_op(&c, LC_PRINT) && has_op(&c, LC_LOAD_CONST)); lc_code_object_free(&c); }

    return ISO_TEST_RESULT();
}
