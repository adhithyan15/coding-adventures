// Tests for the C++ lisp-vm library, using the header-only iso_test.h harness
// (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

#include "lisp_compiler.hpp"
#include "lisp_vm.hpp"

namespace lv = ca::lisp_vm;
namespace lc = ca::lisp_compiler;
using lc::CodeObject;
using lc::Instruction;
using lc::LispOp;
using lc::Value;
using lc::ValueKind;

static Value exec(const CodeObject& code) {
    lv::LispVm vm;
    vm.execute(code);
    return vm.stack.empty() ? Value::nil() : vm.stack.back();
}
static bool is_int(const Value& v, std::int64_t n) {
    return v.kind == ValueKind::Integer && v.integer == n;
}
static bool run_int(const std::string& src, std::int64_t n) {
    return is_int(lv::run(src), n);
}
static bool run_nil(const std::string& src) {
    return lv::run(src).kind == ValueKind::Nil;
}

int main() {
    using I = Instruction;

    // ── stack ops ────────────────────────────────────────────────────────────
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::Halt)},
                           {Value::integer_(42)},
                           {}}),
                     42));
    ISO_CHECK(exec({{I(LispOp::LoadNil), I(LispOp::Halt)}, {}, {}}).kind ==
              ValueKind::Nil);
    {
        Value r = exec({{I(LispOp::LoadTrue), I(LispOp::Halt)}, {}, {}});
        ISO_CHECK(r.kind == ValueKind::Bool && r.boolean);
    }
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::Pop), I(LispOp::Halt)},
                           {Value::integer_(1), Value::integer_(2)},
                           {}}),
                     1));

    // ── variables ────────────────────────────────────────────────────────────
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::StoreName, 0),
                            I(LispOp::LoadName, 0), I(LispOp::Halt)},
                           {Value::integer_(42)},
                           {"x"}}),
                     42));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::StoreLocal, 0),
                            I(LispOp::LoadLocal, 0), I(LispOp::Halt)},
                           {Value::integer_(99)},
                           {}}),
                     99));

    // ── arithmetic ───────────────────────────────────────────────────────────
    struct AC {
        LispOp op;
        std::int64_t a, b, r;
    };
    for (AC c : {AC{LispOp::Add, 3, 4, 7}, AC{LispOp::Sub, 10, 3, 7},
                 AC{LispOp::Mul, 6, 7, 42}, AC{LispOp::Div, 10, 3, 3}})
        ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                                I(c.op), I(LispOp::Halt)},
                               {Value::integer_(c.a), Value::integer_(c.b)},
                               {}}),
                         c.r));

    // ── comparison ───────────────────────────────────────────────────────────
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 0),
                            I(LispOp::CmpEq), I(LispOp::Halt)},
                           {Value::integer_(42)},
                           {}}),
                     1));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::CmpEq), I(LispOp::Halt)},
                           {Value::integer_(1), Value::integer_(2)},
                           {}}),
                     0));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadNil), I(LispOp::LoadNil),
                            I(LispOp::CmpEq), I(LispOp::Halt)},
                           {},
                           {}}),
                     1));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::CmpLt), I(LispOp::Halt)},
                           {Value::integer_(1), Value::integer_(2)},
                           {}}),
                     1));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::CmpGt), I(LispOp::Halt)},
                           {Value::integer_(5), Value::integer_(3)},
                           {}}),
                     1));

    // ── control flow ─────────────────────────────────────────────────────────
    ISO_CHECK(is_int(exec({{I(LispOp::Jump, 2), I(LispOp::LoadConst, 0),
                            I(LispOp::LoadConst, 1), I(LispOp::Halt)},
                           {Value::integer_(99), Value::integer_(42)},
                           {}}),
                     42));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadNil), I(LispOp::JumpIfFalse, 3),
                            I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::Halt)},
                           {Value::integer_(99), Value::integer_(42)},
                           {}}),
                     42));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::JumpIfFalse, 3),
                            I(LispOp::LoadConst, 1), I(LispOp::LoadConst, 2),
                            I(LispOp::Halt)},
                           {Value::integer_(0), Value::integer_(99),
                            Value::integer_(42)},
                           {}}),
                     42));

    // ── cons cells (inspect the heap) ────────────────────────────────────────
    {
        lv::LispVm vm;
        vm.execute({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                     I(LispOp::Cons), I(LispOp::Halt)},
                    {Value::integer_(2), Value::integer_(1)},
                    {}});
        const Value& top = vm.stack.back();
        ISO_CHECK(top.kind == ValueKind::ConsAddr);
        const auto* cell = std::get_if<lv::ConsCell>(&vm.heap[top.addr]);
        ISO_CHECK(cell != nullptr && is_int(cell->car, 1) &&
                  is_int(cell->cdr, 2));
    }
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::Cons), I(LispOp::Car), I(LispOp::Halt)},
                           {Value::integer_(2), Value::integer_(1)},
                           {}}),
                     1));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::Cons), I(LispOp::Cdr), I(LispOp::Halt)},
                           {Value::integer_(2), Value::integer_(1)},
                           {}}),
                     2));

    // ── symbols ──────────────────────────────────────────────────────────────
    {
        lv::LispVm vm;
        vm.execute({{I(LispOp::MakeSymbol, 0), I(LispOp::Halt)},
                    {Value::string_("foo")},
                    {}});
        const auto* s =
            std::get_if<lv::HeapSymbol>(&vm.heap[vm.stack.back().addr]);
        ISO_CHECK(s != nullptr && s->name == "foo");
    }
    ISO_CHECK(is_int(exec({{I(LispOp::MakeSymbol, 0), I(LispOp::MakeSymbol, 0),
                            I(LispOp::CmpEq), I(LispOp::Halt)},
                           {Value::string_("foo")},
                           {}}),
                     1));

    // ── predicates ───────────────────────────────────────────────────────────
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::IsAtom),
                            I(LispOp::Halt)},
                           {Value::integer_(42)},
                           {}}),
                     1));
    ISO_CHECK(is_int(
        exec({{I(LispOp::LoadNil), I(LispOp::IsAtom), I(LispOp::Halt)}, {}, {}}),
        1));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::LoadConst, 1),
                            I(LispOp::Cons), I(LispOp::IsAtom), I(LispOp::Halt)},
                           {Value::integer_(2), Value::integer_(1)},
                           {}}),
                     0));
    ISO_CHECK(is_int(
        exec({{I(LispOp::LoadNil), I(LispOp::IsNil), I(LispOp::Halt)}, {}, {}}),
        1));
    ISO_CHECK(is_int(exec({{I(LispOp::LoadConst, 0), I(LispOp::IsNil),
                            I(LispOp::Halt)},
                           {Value::integer_(42)},
                           {}}),
                     0));

    // ── functions / closures (bytecode) ──────────────────────────────────────
    {
        CodeObject fcode{{I(LispOp::LoadConst, 0), I(LispOp::Return)},
                         {Value::integer_(42)},
                         {}};
        CodeObject mcode{{I(LispOp::LoadConst, 0), I(LispOp::MakeClosure, 0),
                          I(LispOp::CallFunction, 0), I(LispOp::Halt)},
                         {Value::code_(std::make_shared<CodeObject>(fcode))},
                         {}};
        ISO_CHECK(is_int(exec(mcode), 42));
    }
    {
        CodeObject fcode{{I(LispOp::LoadLocal, 0), I(LispOp::LoadLocal, 1),
                          I(LispOp::Add), I(LispOp::Return)},
                         {Value::string_("_p0"), Value::string_("_p1")},
                         {}};
        CodeObject mcode{{I(LispOp::LoadConst, 1), I(LispOp::LoadConst, 2),
                          I(LispOp::LoadConst, 0), I(LispOp::MakeClosure, 2),
                          I(LispOp::CallFunction, 2), I(LispOp::Halt)},
                         {Value::code_(std::make_shared<CodeObject>(fcode)),
                          Value::integer_(3), Value::integer_(4)},
                         {}};
        ISO_CHECK(is_int(exec(mcode), 7));
    }

    // ── print ────────────────────────────────────────────────────────────────
    {
        auto pr = lv::run_with_output("(print 42)");
        ISO_CHECK(!pr.second.empty() &&
                  pr.second[0].find("42") != std::string::npos);
    }
    {
        auto pr = lv::run_with_output("(print nil)");
        ISO_CHECK(!pr.second.empty() &&
                  pr.second[0].find("nil") != std::string::npos);
    }
    {
        lv::LispVm vm;
        vm.execute({{I(LispOp::LoadNil), I(LispOp::LoadConst, 0),
                     I(LispOp::Cons), I(LispOp::LoadConst, 1), I(LispOp::Cons),
                     I(LispOp::Print), I(LispOp::Halt)},
                    {Value::integer_(2), Value::integer_(1)},
                    {}});
        ISO_CHECK(!vm.output.empty() &&
                  vm.output[0].find("(1 2)") != std::string::npos);
    }

    // ── end-to-end ───────────────────────────────────────────────────────────
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
    ISO_CHECK(run_int("(define x 2) (cond ((eq x 1) 10) ((eq x 2) 20) (t 30))",
                      20));
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
    ISO_CHECK(lv::run("t").kind == ValueKind::Bool && lv::run("t").boolean);
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

    // Deep NON-tail recursion must throw (call-stack guard), not overflow the
    // native stack. `(+ 1 (f ...))` is a non-tail call, so it recurses until
    // the depth cap trips.
    {
        bool threw = false;
        try {
            lv::run(
                "(define f (lambda (n) (cond ((eq n 0) 0) (t (+ 1 (f (- n "
                "1))))))) (f 100000)");
        } catch (const lv::VmError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    // Deep TAIL recursion still succeeds (loops, no native growth).
    ISO_CHECK(run_int(
        "(define loop (lambda (n) (cond ((eq n 0) 0) (t (loop (- n 1))))))"
        "(loop 100000)",
        0));

    return ISO_TEST_RESULT();
}
