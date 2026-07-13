/*
 * Tests for lisp-vm, using the header-only iso_test.h harness (pure ISO).
 * Cases mirror the Rust crate's own unit tests: low-level bytecode execution
 * (hand-built CodeObjects) and end-to-end runs of source through the full
 * lexer → parser → compiler → VM pipeline.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h> /* free */
#include <string.h> /* strstr */

#include "lisp_compiler.h"
#include "lisp_vm.h"

/* ── bytecode builders (borrowed strings/code; the code object is never freed
 * via lc_code_object_free, so string literals are safe) ────────────────────*/
static LcInstruction op_(LcOp o) {
    LcInstruction i = {o, 0, 0};
    return i;
}
static LcInstruction opw(LcOp o, size_t operand) {
    LcInstruction i = {o, 1, operand};
    return i;
}
static LcValue vint(int64_t n) {
    LcValue v = {LC_VAL_INTEGER, n, NULL, 0, 0, NULL};
    return v;
}
static LcValue vstr(const char *s) {
    LcValue v = {LC_VAL_STRING, 0, (char *)s, 0, 0, NULL};
    return v;
}
static LcValue vcode(LcCodeObject *c) {
    LcValue v = {LC_VAL_CODE, 0, NULL, 0, 0, c};
    return v;
}
static LcValue vnil(void) {
    LcValue v = {LC_VAL_NIL, 0, NULL, 0, 0, NULL};
    return v;
}

/* Execute `code`, returning a clone of the top of stack (or nil); frees the VM. */
static LcValue exec(const LcCodeObject *code) {
    LispVm *vm = lv_new();
    LvError err;
    lv_execute(vm, code, &err);
    const LcValue *top = lv_stack_top(vm);
    LcValue r = top != NULL ? lc_value_clone(top) : vnil();
    lv_free(vm);
    return r;
}
static int is_int(LcValue v, int64_t n) {
    int ok = v.kind == LC_VAL_INTEGER && v.integer == n;
    lc_value_free(&v);
    return ok;
}

/* end-to-end helpers */
static int run_int(const char *src, int64_t expect) {
    LcValue out;
    LvError err;
    if (!lv_run(src, &out, &err)) return 0;
    int ok = out.kind == LC_VAL_INTEGER && out.integer == expect;
    lc_value_free(&out);
    return ok;
}
static int run_nil(const char *src) {
    LcValue out;
    LvError err;
    if (!lv_run(src, &out, &err)) return 0;
    int ok = out.kind == LC_VAL_NIL;
    lc_value_free(&out);
    return ok;
}
static int run_true(const char *src) {
    LcValue out;
    LvError err;
    if (!lv_run(src, &out, &err)) return 0;
    int ok = out.kind == LC_VAL_BOOL && out.boolean == 1;
    lc_value_free(&out);
    return ok;
}

int main(void) {
    /* ── stack ops ───────────────────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), op_(LC_HALT)};
        LcValue c[] = {vint(42)};
        LcCodeObject code = {i, 2, c, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 42));
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_NIL), op_(LC_HALT)};
        LcCodeObject code = {i, 2, NULL, 0, NULL, 0};
        LcValue r = exec(&code);
        ISO_CHECK(r.kind == LC_VAL_NIL);
        lc_value_free(&r);
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_TRUE), op_(LC_HALT)};
        LcCodeObject code = {i, 2, NULL, 0, NULL, 0};
        LcValue r = exec(&code);
        ISO_CHECK(r.kind == LC_VAL_BOOL && r.boolean == 1);
        lc_value_free(&r);
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_POP), op_(LC_HALT)};
        LcValue c[] = {vint(1), vint(2)};
        LcCodeObject code = {i, 4, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }

    /* ── variables ───────────────────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_STORE_NAME, 0),
                             opw(LC_LOAD_NAME, 0), op_(LC_HALT)};
        LcValue c[] = {vint(42)};
        char *nm[] = {(char *)"x"};
        LcCodeObject code = {i, 4, c, 1, nm, 1};
        ISO_CHECK(is_int(exec(&code), 42));
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_STORE_LOCAL, 0),
                             opw(LC_LOAD_LOCAL, 0), op_(LC_HALT)};
        LcValue c[] = {vint(99)};
        LcCodeObject code = {i, 4, c, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 99));
    }

    /* ── arithmetic ──────────────────────────────────────────────────────────*/
    {
        struct { LcOp op; int64_t a, b, r; } cases[] = {
            {LC_ADD, 3, 4, 7},    {LC_SUB, 10, 3, 7},
            {LC_MUL, 6, 7, 42},   {LC_DIV, 10, 3, 3}};
        for (size_t k = 0; k < 4; k++) {
            LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                                 op_(cases[k].op), op_(LC_HALT)};
            LcValue c[] = {vint(cases[k].a), vint(cases[k].b)};
            LcCodeObject code = {i, 4, c, 2, NULL, 0};
            ISO_CHECK(is_int(exec(&code), cases[k].r));
        }
    }

    /* ── comparison ──────────────────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 0),
                             op_(LC_CMP_EQ), op_(LC_HALT)};
        LcValue c[] = {vint(42)};
        LcCodeObject code = {i, 4, c, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_CMP_EQ), op_(LC_HALT)};
        LcValue c[] = {vint(1), vint(2)};
        LcCodeObject code = {i, 4, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 0));
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_NIL), op_(LC_LOAD_NIL), op_(LC_CMP_EQ),
                             op_(LC_HALT)};
        LcCodeObject code = {i, 4, NULL, 0, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_CMP_LT), op_(LC_HALT)};
        LcValue c[] = {vint(1), vint(2)};
        LcCodeObject code = {i, 4, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_CMP_GT), op_(LC_HALT)};
        LcValue c[] = {vint(5), vint(3)};
        LcCodeObject code = {i, 4, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }

    /* ── control flow ────────────────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_JUMP, 2), opw(LC_LOAD_CONST, 0),
                             opw(LC_LOAD_CONST, 1), op_(LC_HALT)};
        LcValue c[] = {vint(99), vint(42)};
        LcCodeObject code = {i, 4, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 42));
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_NIL), opw(LC_JUMP_IF_FALSE, 3),
                             opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_HALT)};
        LcValue c[] = {vint(99), vint(42)};
        LcCodeObject code = {i, 5, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 42));
    }
    { /* zero is falsy */
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_JUMP_IF_FALSE, 3),
                             opw(LC_LOAD_CONST, 1), opw(LC_LOAD_CONST, 2),
                             op_(LC_HALT)};
        LcValue c[] = {vint(0), vint(99), vint(42)};
        LcCodeObject code = {i, 5, c, 3, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 42));
    }

    /* ── cons cells (inspect the heap) ───────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_CONS), op_(LC_HALT)};
        LcValue c[] = {vint(2), vint(1)};
        LcCodeObject code = {i, 4, c, 2, NULL, 0};
        LispVm *vm = lv_new();
        LvError err;
        ISO_CHECK(lv_execute(vm, &code, &err));
        const LcValue *top = lv_stack_top(vm);
        ISO_CHECK(top != NULL && top->kind == LC_VAL_CONS_ADDR);
        const LvHeapObject *o = lv_heap_at(vm, top->addr);
        ISO_CHECK(o != NULL && lv_heap_kind(o) == LV_CONS);
        ISO_CHECK(lv_cons_car(o)->integer == 1 && lv_cons_cdr(o)->integer == 2);
        lv_free(vm);
    }
    { /* CAR / CDR */
        LcInstruction ic[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                              op_(LC_CONS), op_(LC_CAR), op_(LC_HALT)};
        LcValue cc[] = {vint(2), vint(1)};
        LcCodeObject codec = {ic, 5, cc, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&codec), 1));
        LcInstruction id[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                              op_(LC_CONS), op_(LC_CDR), op_(LC_HALT)};
        LcCodeObject coded = {id, 5, cc, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&coded), 2));
    }

    /* ── symbols ─────────────────────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_MAKE_SYMBOL, 0), op_(LC_HALT)};
        LcValue c[] = {vstr("foo")};
        LcCodeObject code = {i, 2, c, 1, NULL, 0};
        LispVm *vm = lv_new();
        LvError err;
        ISO_CHECK(lv_execute(vm, &code, &err));
        const LcValue *top = lv_stack_top(vm);
        const LvHeapObject *o = lv_heap_at(vm, top->addr);
        ISO_CHECK(o != NULL && lv_heap_kind(o) == LV_SYMBOL &&
                  strcmp(lv_symbol_name(o), "foo") == 0);
        lv_free(vm);
    }
    { /* interning: two MAKE_SYMBOL of the same name are eq */
        LcInstruction i[] = {opw(LC_MAKE_SYMBOL, 0), opw(LC_MAKE_SYMBOL, 0),
                             op_(LC_CMP_EQ), op_(LC_HALT)};
        LcValue c[] = {vstr("foo")};
        LcCodeObject code = {i, 4, c, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }

    /* ── predicates ──────────────────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), op_(LC_IS_ATOM),
                             op_(LC_HALT)};
        LcValue c[] = {vint(42)};
        LcCodeObject code = {i, 3, c, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_NIL), op_(LC_IS_ATOM), op_(LC_HALT)};
        LcCodeObject code = {i, 3, NULL, 0, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), opw(LC_LOAD_CONST, 1),
                             op_(LC_CONS), op_(LC_IS_ATOM), op_(LC_HALT)};
        LcValue c[] = {vint(2), vint(1)};
        LcCodeObject code = {i, 5, c, 2, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 0));
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_NIL), op_(LC_IS_NIL), op_(LC_HALT)};
        LcCodeObject code = {i, 3, NULL, 0, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 1));
    }
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), op_(LC_IS_NIL),
                             op_(LC_HALT)};
        LcValue c[] = {vint(42)};
        LcCodeObject code = {i, 3, c, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&code), 0));
    }

    /* ── functions / closures (bytecode) ─────────────────────────────────────*/
    {
        LcInstruction fi[] = {opw(LC_LOAD_CONST, 0), op_(LC_RETURN)};
        LcValue fc[] = {vint(42)};
        LcCodeObject fcode = {fi, 2, fc, 1, NULL, 0};
        LcInstruction mi[] = {opw(LC_LOAD_CONST, 0), opw(LC_MAKE_CLOSURE, 0),
                              opw(LC_CALL_FUNCTION, 0), op_(LC_HALT)};
        LcValue mc[] = {vcode(&fcode)};
        LcCodeObject mcode = {mi, 4, mc, 1, NULL, 0};
        ISO_CHECK(is_int(exec(&mcode), 42));
    }
    {
        LcInstruction fi[] = {opw(LC_LOAD_LOCAL, 0), opw(LC_LOAD_LOCAL, 1),
                              op_(LC_ADD), op_(LC_RETURN)};
        LcValue fc[] = {vstr("_p0"), vstr("_p1")};
        LcCodeObject fcode = {fi, 4, fc, 2, NULL, 0};
        LcInstruction mi[] = {opw(LC_LOAD_CONST, 1), opw(LC_LOAD_CONST, 2),
                              opw(LC_LOAD_CONST, 0), opw(LC_MAKE_CLOSURE, 2),
                              opw(LC_CALL_FUNCTION, 2), op_(LC_HALT)};
        LcValue mc[] = {vcode(&fcode), vint(3), vint(4)};
        LcCodeObject mcode = {mi, 6, mc, 3, NULL, 0};
        ISO_CHECK(is_int(exec(&mcode), 7));
    }

    /* ── print (collected output) ────────────────────────────────────────────*/
    {
        LcInstruction i[] = {opw(LC_LOAD_CONST, 0), op_(LC_PRINT), op_(LC_HALT)};
        LcValue c[] = {vint(42)};
        LcCodeObject code = {i, 3, c, 1, NULL, 0};
        LispVm *vm = lv_new();
        LvError err;
        lv_execute(vm, &code, &err);
        ISO_CHECK(lv_output_len(vm) == 1 &&
                  strstr(lv_output_at(vm, 0), "42") != NULL);
        lv_free(vm);
    }
    {
        LcInstruction i[] = {op_(LC_LOAD_NIL), op_(LC_PRINT), op_(LC_HALT)};
        LcCodeObject code = {i, 3, NULL, 0, NULL, 0};
        LispVm *vm = lv_new();
        LvError err;
        lv_execute(vm, &code, &err);
        ISO_CHECK(strstr(lv_output_at(vm, 0), "nil") != NULL);
        lv_free(vm);
    }
    { /* print a two-element list "(1 2)" */
        LcInstruction i[] = {op_(LC_LOAD_NIL),        opw(LC_LOAD_CONST, 0),
                             op_(LC_CONS),            opw(LC_LOAD_CONST, 1),
                             op_(LC_CONS),            op_(LC_PRINT),
                             op_(LC_HALT)};
        LcValue c[] = {vint(2), vint(1)};
        LcCodeObject code = {i, 7, c, 2, NULL, 0};
        LispVm *vm = lv_new();
        LvError err;
        lv_execute(vm, &code, &err);
        ISO_CHECK(strstr(lv_output_at(vm, 0), "(1 2)") != NULL);
        lv_free(vm);
    }

    /* ── end-to-end (source → result) ────────────────────────────────────────*/
    ISO_CHECK(run_int("(+ 1 2)", 3));
    ISO_CHECK(run_int("(- 10 3)", 7));
    ISO_CHECK(run_int("(* 4 5)", 20));
    ISO_CHECK(run_int("(/ 10 2)", 5));
    ISO_CHECK(run_int("(+ (* 2 3) (- 10 4))", 12));
    ISO_CHECK(run_int("(* (+ 1 2) (+ 3 4))", 21));
    ISO_CHECK(run_int("(eq 1 1)", 1));
    ISO_CHECK(run_int("(eq 1 2)", 0));
    ISO_CHECK(run_int("(< 1 2)", 1));
    ISO_CHECK(run_int("(> 3 2)", 1));
    ISO_CHECK(run_int("(define x 42) x", 42));
    ISO_CHECK(run_int("(define x (+ 1 2)) x", 3));
    ISO_CHECK(run_int("(define x 10) (define y 20) (+ x y)", 30));
    ISO_CHECK(run_int("(cond ((eq 1 1) 42) (t 0))", 42));
    ISO_CHECK(run_int("(cond ((eq 1 2) 42) (t 99))", 99));
    ISO_CHECK(run_int("(define x 2) (cond ((eq x 1) 10) ((eq x 2) 20) (t 30))", 20));
    ISO_CHECK(run_int("(cond (t 42))", 42));
    ISO_CHECK(run_int("((lambda (x) x) 42)", 42));
    ISO_CHECK(run_int("((lambda (x) (+ x 1)) 41)", 42));
    ISO_CHECK(run_int("((lambda (x y) (+ x y)) 10 20)", 30));
    ISO_CHECK(run_int("(define double (lambda (x) (* x 2))) (double 21)", 42));
    ISO_CHECK(run_int(
        "(define y 10) (define add-y (lambda (x) (+ x y))) (add-y 32)", 42));
    ISO_CHECK(run_int("(car (cons 1 2))", 1));
    ISO_CHECK(run_int("(cdr (cons 1 2))", 2));
    ISO_CHECK(run_int("(car (cdr (cons 1 (cons 2 3))))", 2));
    ISO_CHECK(run_int("(quote 42)", 42));
    ISO_CHECK(run_nil("(quote nil)"));
    ISO_CHECK(run_int("'42", 42));
    ISO_CHECK(run_int("(car (quote (1 2 3)))", 1));
    ISO_CHECK(run_int("(car (cdr (quote (1 2 3))))", 2));
    ISO_CHECK(run_nil("(quote ())"));
    ISO_CHECK(run_int("(atom 42)", 1));
    ISO_CHECK(run_int("(atom (cons 1 2))", 0));
    ISO_CHECK(run_int("(is-nil nil)", 1));
    ISO_CHECK(run_int("(is-nil 42)", 0));
    ISO_CHECK(run_nil("nil"));
    ISO_CHECK(run_true("t"));
    ISO_CHECK(run_nil("()"));
    ISO_CHECK(run_int(
        "(define factorial (lambda (n) (cond ((eq n 0) 1) (t (* n (factorial "
        "(- n 1))))))) (factorial 5)",
        120));
    ISO_CHECK(run_int(
        "(define factorial (lambda (n) (cond ((eq n 0) 1) (t (* n (factorial "
        "(- n 1))))))) (factorial 10)",
        3628800));
    ISO_CHECK(run_int(
        "(define fib (lambda (n) (cond ((eq n 0) 0) ((eq n 1) 1) (t (+ (fib "
        "(- n 1)) (fib (- n 2))))))) (fib 10)",
        55));
    ISO_CHECK(run_int(
        "(define fi (lambda (n acc) (cond ((eq n 0) acc) (t (fi (- n 1) (* n "
        "acc)))))) (fi 10 1)",
        3628800));
    ISO_CHECK(run_int(
        "(define countdown (lambda (n) (cond ((eq n 0) 0) (t (countdown (- n "
        "1)))))) (countdown 10000)",
        0));
    ISO_CHECK(run_int("(eq (quote foo) (quote foo))", 1));
    ISO_CHECK(run_int("(eq (quote foo) (quote bar))", 0));
    ISO_CHECK(run_int(
        "(define apply-to-5 (lambda (f) (f 5))) (define double (lambda (x) (* "
        "x 2))) (apply-to-5 double)",
        10));
    ISO_CHECK(run_int(
        "(define make-adder (lambda (x) (lambda (y) (+ x y)))) (define add-10 "
        "(make-adder 10)) (add-10 32)",
        42));

    /* Deep NON-tail recursion must fail cleanly (call-stack guard), not
     * overflow the native C stack. `(+ 1 (f ...))` is a non-tail call, so this
     * recurses natively until the depth cap trips. */
    {
        LcValue out;
        LvError err;
        int ok = lv_run(
            "(define f (lambda (n) (cond ((eq n 0) 0) (t (+ 1 (f (- n 1)))))))"
            "(f 100000)",
            &out, &err);
        ISO_CHECK_MSG(ok == 0, "deep non-tail recursion should error, not crash");
        if (ok) lc_value_free(&out);
    }
    /* Deep TAIL recursion, by contrast, still succeeds (loops, no growth). */
    ISO_CHECK(run_int(
        "(define loop (lambda (n) (cond ((eq n 0) 0) (t (loop (- n 1))))))"
        "(loop 100000)",
        0));

    return ISO_TEST_RESULT();
}
