// lisp_compiler.hpp — compile S-expression ASTs into Lisp bytecode.
// ==================================================================
//
// A faithful, header-only port of the Rust `lisp-compiler` crate (namespace
// `ca::lisp_compiler`). It sits on top of the header-only `lisp-parser` (and
// `lisp-lexer`): Lisp source in, a `CodeObject` of bytecode out. The compiler
// inspects the first element of each list to assign meaning — `define`,
// `lambda`, `cond`, `quote`, arithmetic, comparison, cons cells, predicates, or
// an ordinary call — and tracks tail position so tail calls emit `TailCall`.
//
// Rust's `Result`/`CompileError` becomes a thrown `CompileError`. Nested lambda
// bodies are `CodeObject`s stored as `Value::Code` constants (shared for cheap
// copies; equality is still structural).
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef LISP_COMPILER_HPP
#define LISP_COMPILER_HPP

#include <cstddef>
#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

#include "lisp_parser.hpp"

namespace ca::lisp_compiler {

// Lisp bytecode opcodes (one byte each; the high nibble groups by category).
enum class LispOp : std::uint8_t {
    LoadConst = 0x01,
    Pop = 0x02,
    LoadNil = 0x03,
    LoadTrue = 0x04,
    StoreName = 0x10,
    LoadName = 0x11,
    StoreLocal = 0x12,
    LoadLocal = 0x13,
    Add = 0x20,
    Sub = 0x21,
    Mul = 0x22,
    Div = 0x23,
    CmpEq = 0x30,
    CmpLt = 0x31,
    CmpGt = 0x32,
    Jump = 0x40,
    JumpIfFalse = 0x41,
    JumpIfTrue = 0x42,
    MakeClosure = 0x50,
    CallFunction = 0x51,
    TailCall = 0x52,
    Return = 0x53,
    Cons = 0x70,
    Car = 0x71,
    Cdr = 0x72,
    MakeSymbol = 0x73,
    IsAtom = 0x74,
    IsNil = 0x75,
    Print = 0xA0,
    Halt = 0xFF
};

// A single instruction: an opcode and an optional operand.
struct Instruction {
    LispOp opcode;
    std::optional<std::size_t> operand;

    explicit Instruction(LispOp op) : opcode(op) {}
    Instruction(LispOp op, std::size_t operand) : opcode(op), operand(operand) {}

    friend bool operator==(const Instruction& a, const Instruction& b) {
        return a.opcode == b.opcode && a.operand == b.operand;
    }
};

enum class ValueKind {
    Integer,
    String,
    Bool,
    Nil,
    Symbol,
    ConsAddr,
    ClosureAddr,
    Code
};

struct CodeObject;

// A runtime value: on the stack, in the constant pool, or in a variable.
struct Value {
    ValueKind kind = ValueKind::Nil;
    std::int64_t integer = 0;
    std::string str;  // String / Symbol
    bool boolean = false;
    std::size_t addr = 0;
    std::shared_ptr<CodeObject> code;  // Code (shared for cheap copies)

    static Value integer_(std::int64_t v) {
        Value x;
        x.kind = ValueKind::Integer;
        x.integer = v;
        return x;
    }
    static Value string_(std::string v) {
        Value x;
        x.kind = ValueKind::String;
        x.str = std::move(v);
        return x;
    }
    static Value symbol(std::string v) {
        Value x;
        x.kind = ValueKind::Symbol;
        x.str = std::move(v);
        return x;
    }
    static Value boolean_(bool v) {
        Value x;
        x.kind = ValueKind::Bool;
        x.boolean = v;
        return x;
    }
    static Value nil() { return Value{}; }
    static Value cons_addr(std::size_t a) {
        Value x;
        x.kind = ValueKind::ConsAddr;
        x.addr = a;
        return x;
    }
    static Value closure_addr(std::size_t a) {
        Value x;
        x.kind = ValueKind::ClosureAddr;
        x.addr = a;
        return x;
    }
    static Value code_(std::shared_ptr<CodeObject> c) {
        Value x;
        x.kind = ValueKind::Code;
        x.code = std::move(c);
        return x;
    }

    // Falsy: Nil, Bool(false), Integer(0).
    bool is_falsy() const {
        return kind == ValueKind::Nil ||
               (kind == ValueKind::Bool && !boolean) ||
               (kind == ValueKind::Integer && integer == 0);
    }

    bool operator==(const Value& o) const;  // defined after CodeObject
    bool operator!=(const Value& o) const { return !(*this == o); }
};

// A compiled unit of Lisp code.
struct CodeObject {
    std::vector<Instruction> instructions;
    std::vector<Value> constants;
    std::vector<std::string> names;

    friend bool operator==(const CodeObject& a, const CodeObject& b) {
        return a.instructions == b.instructions && a.constants == b.constants &&
               a.names == b.names;
    }
};

inline bool Value::operator==(const Value& o) const {
    if (kind != o.kind) return false;
    switch (kind) {
        case ValueKind::Integer: return integer == o.integer;
        case ValueKind::String:
        case ValueKind::Symbol: return str == o.str;
        case ValueKind::Bool: return boolean == o.boolean;
        case ValueKind::Nil: return true;
        case ValueKind::ConsAddr:
        case ValueKind::ClosureAddr: return addr == o.addr;
        case ValueKind::Code: return *code == *o.code;
    }
    return false;
}

// Thrown on a compile (or wrapped parse) error.
class CompileError : public std::runtime_error {
   public:
    explicit CompileError(const std::string& message)
        : std::runtime_error(message) {}
};

namespace detail {

namespace lp = ca::lisp_parser;

class Compiler {
   public:
    CodeObject compile_program(const std::vector<lp::SExpr>& program) {
        for (std::size_t i = 0; i < program.size(); ++i) {
            compile_sexpr(program[i]);
            if (i + 1 < program.size()) emit(LispOp::Pop);
        }
        emit(LispOp::Halt);
        return CodeObject{std::move(instructions_), std::move(constants_),
                          std::move(names_)};
    }

   private:
    // -- emit / pool helpers --
    void emit(LispOp op) { instructions_.emplace_back(op); }
    void emit_with(LispOp op, std::size_t operand) {
        instructions_.emplace_back(op, operand);
    }
    std::size_t emit_jump(LispOp op) {
        std::size_t idx = instructions_.size();
        instructions_.emplace_back(op, std::size_t{0});
        return idx;
    }
    void patch_jump(std::size_t idx) {
        instructions_[idx].operand = instructions_.size();
    }
    std::size_t add_constant(const Value& v) {
        for (std::size_t i = 0; i < constants_.size(); ++i)
            if (constants_[i] == v) return i;
        constants_.push_back(v);
        return constants_.size() - 1;
    }
    std::size_t add_name(const std::string& name) {
        for (std::size_t i = 0; i < names_.size(); ++i)
            if (names_[i] == name) return i;
        names_.push_back(name);
        return names_.size() - 1;
    }
    std::optional<std::size_t> get_local(const std::string& name) const {
        if (!scopes_.empty()) {
            const auto& scope = scopes_.back();
            auto it = scope.find(name);
            if (it != scope.end()) return it->second;
        }
        return std::nullopt;
    }

    // -- AST helpers --
    static bool is_symbol(const lp::SExpr& e, const char* name) {
        return e.kind() == lp::SExprKind::Atom &&
               e.atom_kind() == lp::AtomKind::Symbol && e.atom_value() == name;
    }
    // The elements of a List / DottedPair (for dotted, plus the final cdr).
    static std::vector<const lp::SExpr*> children(const lp::SExpr& e) {
        std::vector<const lp::SExpr*> out;
        std::size_t n = e.child_count();
        for (std::size_t i = 0; i < n; ++i) out.push_back(&e.child(i));
        if (e.kind() == lp::SExprKind::DottedPair)
            out.push_back(&e.dotted_last());
        return out;
    }
    static std::int64_t parse_int(const std::string& s, const char* ctx) {
        try {
            std::size_t pos = 0;
            long long v = std::stoll(s, &pos);
            if (pos != s.size()) throw std::invalid_argument("trailing");
            return static_cast<std::int64_t>(v);
        } catch (const std::exception&) {
            throw CompileError(std::string("CompileError: ") + ctx + s);
        }
    }
    static std::string strip_quotes(const std::string& v) {
        if (v.size() >= 2 && v.front() == '"' && v.back() == '"')
            return v.substr(1, v.size() - 2);
        return v;
    }

    // -- compilation --
    void compile_sexpr(const lp::SExpr& e) {
        switch (e.kind()) {
            case lp::SExprKind::Atom:
                compile_atom(e.atom_kind(), e.atom_value());
                break;
            case lp::SExprKind::List:
            case lp::SExprKind::DottedPair:
                compile_list(children(e));
                break;
            case lp::SExprKind::Quoted:
                compile_quoted_datum(e.quoted_inner());
                break;
        }
    }

    void compile_atom(lp::AtomKind kind, const std::string& value) {
        if (kind == lp::AtomKind::Number) {
            emit_with(LispOp::LoadConst,
                      add_constant(Value::integer_(
                          parse_int(value, "Invalid number: "))));
        } else if (kind == lp::AtomKind::String) {
            emit_with(LispOp::LoadConst,
                      add_constant(Value::string_(strip_quotes(value))));
        } else {  // Symbol
            if (value == "nil") {
                emit(LispOp::LoadNil);
            } else if (value == "t") {
                emit(LispOp::LoadTrue);
            } else if (auto slot = get_local(value)) {
                emit_with(LispOp::LoadLocal, *slot);
            } else {
                emit_with(LispOp::LoadName, add_name(value));
            }
        }
    }

    void compile_list(const std::vector<const lp::SExpr*>& e) {
        if (e.empty()) {
            emit(LispOp::LoadNil);
            return;
        }
        const lp::SExpr& first = *e[0];
        if (first.kind() == lp::SExprKind::Atom &&
            first.atom_kind() == lp::AtomKind::Symbol) {
            const std::string& sym = first.atom_value();
            if (sym == "define") return compile_define(e);
            if (sym == "lambda") return compile_lambda(e);
            if (sym == "cond") return compile_cond(e);
            if (sym == "quote") return compile_quote_form(e);
            if (sym == "cons") return compile_cons(e);
            if (sym == "car") return compile_unary_op(e, LispOp::Car);
            if (sym == "cdr") return compile_unary_op(e, LispOp::Cdr);
            if (sym == "atom") return compile_unary_op(e, LispOp::IsAtom);
            if (sym == "eq") return compile_binary_op(e, LispOp::CmpEq);
            if (sym == "print") return compile_unary_op(e, LispOp::Print);
            if (sym == "is-nil") return compile_unary_op(e, LispOp::IsNil);
            if (auto op = arithmetic_op(sym)) return compile_binary_op(e, *op);
            if (auto op = comparison_op(sym)) return compile_binary_op(e, *op);
        }
        compile_call(e);
    }

    static std::optional<LispOp> arithmetic_op(const std::string& s) {
        if (s == "+") return LispOp::Add;
        if (s == "-") return LispOp::Sub;
        if (s == "*") return LispOp::Mul;
        if (s == "/") return LispOp::Div;
        return std::nullopt;
    }
    static std::optional<LispOp> comparison_op(const std::string& s) {
        if (s == "=") return LispOp::CmpEq;
        if (s == "<") return LispOp::CmpLt;
        if (s == ">") return LispOp::CmpGt;
        return std::nullopt;
    }

    void compile_define(const std::vector<const lp::SExpr*>& e) {
        if (e.size() != 3)
            throw CompileError("CompileError: define expects 2 arguments");
        if (!(e[1]->kind() == lp::SExprKind::Atom &&
              e[1]->atom_kind() == lp::AtomKind::Symbol))
            throw CompileError("CompileError: define name must be a symbol");
        std::string name = e[1]->atom_value();
        bool saved = tail_position_;
        tail_position_ = false;
        compile_sexpr(*e[2]);
        tail_position_ = saved;
        emit_with(LispOp::StoreName, add_name(name));
        emit(LispOp::LoadNil);
    }

    void compile_lambda(const std::vector<const lp::SExpr*>& e) {
        if (e.size() < 3)
            throw CompileError("CompileError: lambda needs params and body");
        if (e[1]->kind() != lp::SExprKind::List)
            throw CompileError("CompileError: lambda params must be a list");
        std::vector<std::string> params;
        std::size_t np = e[1]->child_count();
        for (std::size_t i = 0; i < np; ++i) {
            const lp::SExpr& p = e[1]->child(i);
            if (!(p.kind() == lp::SExprKind::Atom &&
                  p.atom_kind() == lp::AtomKind::Symbol))
                throw CompileError(
                    "CompileError: lambda parameter must be a symbol");
            params.push_back(p.atom_value());
        }

        std::unordered_map<std::string, std::size_t> scope;
        for (std::size_t i = 0; i < params.size(); ++i) scope[params[i]] = i;
        scopes_.push_back(std::move(scope));

        auto saved_instr = std::move(instructions_);
        auto saved_const = std::move(constants_);
        auto saved_names = std::move(names_);
        instructions_.clear();
        constants_.clear();
        names_.clear();
        bool st = tail_position_, sf = in_function_;
        in_function_ = true;

        for (std::size_t i = 2; i < e.size(); ++i) {
            bool is_last = (i == e.size() - 1);
            tail_position_ = is_last;
            compile_sexpr(*e[i]);
            if (!is_last) emit(LispOp::Pop);
        }
        emit(LispOp::Return);
        for (const auto& n : params) constants_.push_back(Value::string_(n));

        auto body = std::make_shared<CodeObject>(
            CodeObject{std::move(instructions_), std::move(constants_),
                       std::move(names_)});

        instructions_ = std::move(saved_instr);
        constants_ = std::move(saved_const);
        names_ = std::move(saved_names);
        tail_position_ = st;
        in_function_ = sf;
        scopes_.pop_back();

        emit_with(LispOp::LoadConst, add_constant(Value::code_(std::move(body))));
        emit_with(LispOp::MakeClosure, params.size());
    }

    void compile_cond(const std::vector<const lp::SExpr*>& e) {
        std::vector<std::size_t> end_jumps;
        for (std::size_t ci = 1; ci < e.size(); ++ci) {
            const lp::SExpr& clause = *e[ci];
            if (clause.kind() != lp::SExprKind::List)
                throw CompileError("CompileError: cond clause must be a list");
            std::size_t parts = clause.child_count();
            if (parts < 2)
                throw CompileError(
                    "CompileError: cond clause needs predicate and expression");
            const lp::SExpr& predicate = clause.child(0);
            const lp::SExpr& expression = clause.child(parts - 1);

            if (is_symbol(predicate, "t")) {
                bool saved = tail_position_;
                compile_sexpr(expression);
                tail_position_ = saved;
            } else {
                bool saved = tail_position_;
                tail_position_ = false;
                compile_sexpr(predicate);
                tail_position_ = saved;

                std::size_t false_jump = emit_jump(LispOp::JumpIfFalse);
                bool saved2 = tail_position_;
                compile_sexpr(expression);
                tail_position_ = saved2;
                end_jumps.push_back(emit_jump(LispOp::Jump));
                patch_jump(false_jump);
            }
        }
        bool has_else = false;
        if (e.size() > 1) {
            const lp::SExpr& last = *e.back();
            if (last.kind() == lp::SExprKind::List && last.child_count() > 0)
                has_else = is_symbol(last.child(0), "t");
        }
        if (e.size() == 1 || !has_else) emit(LispOp::LoadNil);
        for (std::size_t j : end_jumps) patch_jump(j);
    }

    void compile_quote_form(const std::vector<const lp::SExpr*>& e) {
        if (e.size() != 2)
            throw CompileError("CompileError: quote takes exactly 1 argument");
        compile_quoted_datum(*e[1]);
    }

    void compile_quoted_datum(const lp::SExpr& e) {
        switch (e.kind()) {
            case lp::SExprKind::Atom: {
                lp::AtomKind ak = e.atom_kind();
                const std::string& value = e.atom_value();
                if (ak == lp::AtomKind::Number) {
                    emit_with(LispOp::LoadConst,
                              add_constant(Value::integer_(parse_int(
                                  value, "Invalid number in quote: "))));
                } else if (ak == lp::AtomKind::String) {
                    emit_with(LispOp::LoadConst, add_constant(Value::string_(
                                                     strip_quotes(value))));
                } else {
                    if (value == "nil")
                        emit(LispOp::LoadNil);
                    else
                        emit_with(LispOp::MakeSymbol,
                                  add_constant(Value::string_(value)));
                }
                break;
            }
            case lp::SExprKind::List: {
                emit(LispOp::LoadNil);
                std::size_t n = e.child_count();
                for (std::size_t i = n; i > 0; --i) {
                    compile_quoted_datum(e.child(i - 1));
                    emit(LispOp::Cons);
                }
                break;
            }
            case lp::SExprKind::DottedPair: {
                compile_quoted_datum(e.dotted_last());
                std::size_t n = e.child_count();
                for (std::size_t i = n; i > 0; --i) {
                    compile_quoted_datum(e.child(i - 1));
                    emit(LispOp::Cons);
                }
                break;
            }
            case lp::SExprKind::Quoted:
                compile_quoted_datum(e.quoted_inner());
                break;
        }
    }

    void compile_cons(const std::vector<const lp::SExpr*>& e) {
        if (e.size() != 3)
            throw CompileError("CompileError: cons takes exactly 2 arguments");
        bool saved = tail_position_;
        tail_position_ = false;
        compile_sexpr(*e[2]);  // cdr
        compile_sexpr(*e[1]);  // car
        tail_position_ = saved;
        emit(LispOp::Cons);
    }

    void compile_unary_op(const std::vector<const lp::SExpr*>& e, LispOp op) {
        if (e.size() != 2)
            throw CompileError("CompileError: Unary op expects 1 argument");
        bool saved = tail_position_;
        tail_position_ = false;
        compile_sexpr(*e[1]);
        tail_position_ = saved;
        emit(op);
    }

    void compile_binary_op(const std::vector<const lp::SExpr*>& e, LispOp op) {
        if (e.size() != 3)
            throw CompileError("CompileError: Binary op expects 2 arguments");
        bool saved = tail_position_;
        tail_position_ = false;
        compile_sexpr(*e[1]);
        compile_sexpr(*e[2]);
        tail_position_ = saved;
        emit(op);
    }

    void compile_call(const std::vector<const lp::SExpr*>& e) {
        std::size_t argc = e.size() - 1;
        bool saved = tail_position_;
        tail_position_ = false;
        for (std::size_t i = 1; i < e.size(); ++i) compile_sexpr(*e[i]);
        compile_sexpr(*e[0]);
        tail_position_ = saved;
        if (tail_position_ && in_function_)
            emit_with(LispOp::TailCall, argc);
        else
            emit_with(LispOp::CallFunction, argc);
    }

    std::vector<Instruction> instructions_;
    std::vector<Value> constants_;
    std::vector<std::string> names_;
    bool tail_position_ = false;
    bool in_function_ = false;
    std::vector<std::unordered_map<std::string, std::size_t>> scopes_;
};

}  // namespace detail

// Compile a pre-parsed program into a CodeObject.
inline CodeObject compile_ast(const std::vector<lisp_parser::SExpr>& program) {
    detail::Compiler compiler;
    return compiler.compile_program(program);
}

// Compile Lisp source into a CodeObject. Throws CompileError on a parse or
// compile error.
inline CodeObject compile(const std::string& source) {
    try {
        return compile_ast(lisp_parser::parse(source));
    } catch (const lisp_parser::ParseError& e) {
        throw CompileError(std::string("CompileError: Parse error: ") +
                           e.what());
    }
}

}  // namespace ca::lisp_compiler

#endif  // LISP_COMPILER_HPP
