// Tests for the C++ lisp-compiler library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <algorithm>
#include <cstdint>
#include <string>
#include <vector>

#include "lisp_compiler.hpp"

namespace lc = ca::lisp_compiler;
using lc::CodeObject;
using lc::LispOp;
using lc::ValueKind;

static bool has_op(const CodeObject& c, LispOp op) {
    for (const auto& i : c.instructions)
        if (i.opcode == op) return true;
    return false;
}
static std::size_t count_op(const CodeObject& c, LispOp op) {
    std::size_t n = 0;
    for (const auto& i : c.instructions)
        if (i.opcode == op) ++n;
    return n;
}
static bool const_has_int(const CodeObject& c, std::int64_t v) {
    for (const auto& k : c.constants)
        if (k.kind == ValueKind::Integer && k.integer == v) return true;
    return false;
}
static bool const_has_str(const CodeObject& c, const std::string& s) {
    for (const auto& k : c.constants)
        if (k.kind == ValueKind::String && k.str == s) return true;
    return false;
}
static bool name_has(const CodeObject& c, const std::string& s) {
    return std::find(c.names.begin(), c.names.end(), s) != c.names.end();
}
static const CodeObject* first_code_const(const CodeObject& c) {
    for (const auto& k : c.constants)
        if (k.kind == ValueKind::Code) return k.code.get();
    return nullptr;
}
static std::size_t count_code_const(const CodeObject& c) {
    std::size_t n = 0;
    for (const auto& k : c.constants)
        if (k.kind == ValueKind::Code) ++n;
    return n;
}
static std::size_t op_operand(const CodeObject& c, LispOp op) {
    for (const auto& i : c.instructions)
        if (i.opcode == op) return i.operand.value_or(0);
    return 0;
}

int main() {
    // ── atoms ────────────────────────────────────────────────────────────────
    ISO_CHECK(const_has_int(lc::compile("42"), 42) &&
              has_op(lc::compile("42"), LispOp::LoadConst));
    ISO_CHECK(const_has_int(lc::compile("-7"), -7));
    ISO_CHECK(has_op(lc::compile("nil"), LispOp::LoadNil));
    ISO_CHECK(has_op(lc::compile("t"), LispOp::LoadTrue));
    ISO_CHECK(name_has(lc::compile("x"), "x") &&
              has_op(lc::compile("x"), LispOp::LoadName));
    ISO_CHECK(const_has_str(lc::compile("\"hello\""), "hello"));

    // ── arithmetic ───────────────────────────────────────────────────────────
    ISO_CHECK(has_op(lc::compile("(+ 1 2)"), LispOp::LoadConst) &&
              has_op(lc::compile("(+ 1 2)"), LispOp::Add));
    ISO_CHECK(has_op(lc::compile("(- 5 3)"), LispOp::Sub));
    ISO_CHECK(has_op(lc::compile("(* 4 5)"), LispOp::Mul));
    ISO_CHECK(has_op(lc::compile("(/ 10 2)"), LispOp::Div));
    {
        auto c = lc::compile("(+ (* 2 3) 4)");
        ISO_CHECK(has_op(c, LispOp::Mul) && has_op(c, LispOp::Add));
    }
    {
        auto c = lc::compile("(+ 1 2)");
        ISO_CHECK(const_has_int(c, 1) && const_has_int(c, 2));
    }

    // ── comparison ───────────────────────────────────────────────────────────
    ISO_CHECK(has_op(lc::compile("(eq 1 2)"), LispOp::CmpEq));
    ISO_CHECK(has_op(lc::compile("(< 1 2)"), LispOp::CmpLt));
    ISO_CHECK(has_op(lc::compile("(> 3 2)"), LispOp::CmpGt));
    ISO_CHECK(has_op(lc::compile("(= 1 1)"), LispOp::CmpEq));

    // ── define ───────────────────────────────────────────────────────────────
    {
        auto c = lc::compile("(define x 42)");
        ISO_CHECK(name_has(c, "x") && const_has_int(c, 42) &&
                  has_op(c, LispOp::StoreName));
    }
    ISO_CHECK(has_op(lc::compile("(define x 42)"), LispOp::LoadNil));

    // ── cons / car / cdr ─────────────────────────────────────────────────────
    ISO_CHECK(has_op(lc::compile("(cons 1 2)"), LispOp::Cons));
    ISO_CHECK(has_op(lc::compile("(car x)"), LispOp::Car));
    ISO_CHECK(has_op(lc::compile("(cdr x)"), LispOp::Cdr));

    // ── predicates ───────────────────────────────────────────────────────────
    ISO_CHECK(has_op(lc::compile("(atom x)"), LispOp::IsAtom));
    ISO_CHECK(has_op(lc::compile("(is-nil x)"), LispOp::IsNil));

    // ── quote ────────────────────────────────────────────────────────────────
    ISO_CHECK(has_op(lc::compile("(quote 42)"), LispOp::LoadConst));
    {
        auto c = lc::compile("(quote foo)");
        ISO_CHECK(const_has_str(c, "foo") && has_op(c, LispOp::MakeSymbol));
    }
    ISO_CHECK(has_op(lc::compile("(quote nil)"), LispOp::LoadNil));
    {
        auto c = lc::compile("(quote (1 2 3))");
        ISO_CHECK(has_op(c, LispOp::LoadNil) && count_op(c, LispOp::Cons) == 3);
    }
    ISO_CHECK(has_op(lc::compile("(quote ())"), LispOp::LoadNil));
    ISO_CHECK(has_op(lc::compile("'foo"), LispOp::MakeSymbol));
    ISO_CHECK(count_op(lc::compile("'(1 2)"), LispOp::Cons) == 2);

    // ── cond ─────────────────────────────────────────────────────────────────
    {
        auto c = lc::compile("(cond ((eq 1 1) 42) (t 0))");
        ISO_CHECK(has_op(c, LispOp::JumpIfFalse) && has_op(c, LispOp::Jump));
    }
    ISO_CHECK(has_op(lc::compile("(cond (t 42))"), LispOp::LoadConst));

    // ── lambda ───────────────────────────────────────────────────────────────
    {
        auto c = lc::compile("(lambda (x) x)");
        ISO_CHECK(has_op(c, LispOp::LoadConst) &&
                  has_op(c, LispOp::MakeClosure));
    }
    ISO_CHECK(count_code_const(lc::compile("(lambda (x) x)")) == 1);
    {
        auto c = lc::compile("(lambda (x) x)");
        const CodeObject* b = first_code_const(c);
        ISO_CHECK(b != nullptr && has_op(*b, LispOp::LoadLocal));
    }
    {
        auto c = lc::compile("(lambda (x) x)");
        const CodeObject* b = first_code_const(c);
        ISO_CHECK(b != nullptr && has_op(*b, LispOp::Return));
    }
    {
        auto c = lc::compile("(lambda (a b c) a)");
        ISO_CHECK(count_op(c, LispOp::MakeClosure) == 1 &&
                  op_operand(c, LispOp::MakeClosure) == 3);
    }

    // ── function calls ───────────────────────────────────────────────────────
    ISO_CHECK(has_op(lc::compile("(f 1 2)"), LispOp::CallFunction));
    {
        auto c = lc::compile("(f 1 2 3)");
        ISO_CHECK(count_op(c, LispOp::CallFunction) == 1 &&
                  op_operand(c, LispOp::CallFunction) == 3);
    }

    // ── tail calls ───────────────────────────────────────────────────────────
    {
        auto c = lc::compile("(lambda (n) (f n))");
        const CodeObject* b = first_code_const(c);
        ISO_CHECK(b != nullptr && has_op(*b, LispOp::TailCall) &&
                  !has_op(*b, LispOp::CallFunction));
    }
    {
        auto c = lc::compile("(f 1)");
        ISO_CHECK(has_op(c, LispOp::CallFunction) &&
                  !has_op(c, LispOp::TailCall));
    }
    {
        auto c = lc::compile("(lambda (n) (g (f n)))");
        const CodeObject* b = first_code_const(c);
        ISO_CHECK(b != nullptr && has_op(*b, LispOp::TailCall) &&
                  has_op(*b, LispOp::CallFunction));
    }
    {
        auto c = lc::compile("(lambda (n) (cond ((eq n 0) 1) (t (f n))))");
        const CodeObject* b = first_code_const(c);
        ISO_CHECK(b != nullptr && has_op(*b, LispOp::TailCall));
    }

    // ── programs ─────────────────────────────────────────────────────────────
    ISO_CHECK(count_op(lc::compile("1 2 3"), LispOp::Pop) == 2);
    {
        auto c = lc::compile("(define x 5) x");
        ISO_CHECK(name_has(c, "x") && has_op(c, LispOp::StoreName) &&
                  has_op(c, LispOp::LoadName));
    }
    {
        auto c = lc::compile("");
        ISO_CHECK(c.instructions.size() == 1 &&
                  c.instructions[0].opcode == LispOp::Halt);
    }
    ISO_CHECK(has_op(lc::compile("()"), LispOp::LoadNil));

    // ── print ────────────────────────────────────────────────────────────────
    {
        auto c = lc::compile("(print 42)");
        ISO_CHECK(has_op(c, LispOp::Print) && has_op(c, LispOp::LoadConst));
    }

    return ISO_TEST_RESULT();
}
