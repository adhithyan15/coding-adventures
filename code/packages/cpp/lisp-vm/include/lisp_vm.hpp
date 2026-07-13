// lisp_vm.hpp — execute compiled Lisp bytecode.
// ==============================================
//
// A faithful, header-only port of the Rust `lisp-vm` crate (namespace
// `ca::lisp_vm`) — the last stage of the Lisp toolchain (lexer → parser →
// compiler → VM). It executes the bytecode (`lisp_compiler::CodeObject`): a
// value stack, a global variable table, local slots, a grow-only heap (cons
// cells, interned symbols, closures), closures with captured environments, and
// tail-call optimisation.
//
// Values are `lisp_compiler::Value`s (cheaply copyable), so the VM leans on
// value semantics — no manual memory management. `run` runs the full pipeline
// from source and throws `VmError` on a compile or runtime error.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no compiler extensions.
#ifndef LISP_VM_HPP
#define LISP_VM_HPP

#include <cstddef>
#include <cstdio>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <utility>
#include <variant>
#include <vector>

#include "lisp_compiler.hpp"

namespace ca::lisp_vm {

namespace lc = ca::lisp_compiler;
using lc::CodeObject;
using lc::Instruction;
using lc::LispOp;
using lc::Value;
using lc::ValueKind;

// ── Heap objects ─────────────────────────────────────────────────────────────

struct ConsCell {
    Value car;
    Value cdr;
};
struct HeapSymbol {
    std::string name;
};
struct LispClosure {
    CodeObject code;
    std::unordered_map<std::string, Value> env;
    std::vector<std::string> params;
};
using HeapObject = std::variant<ConsCell, HeapSymbol, LispClosure>;

// ── Error ────────────────────────────────────────────────────────────────────

class VmError : public std::runtime_error {
   public:
    explicit VmError(const std::string& message) : std::runtime_error(message) {}
};

// ── The VM ───────────────────────────────────────────────────────────────────

class LispVm {
   public:
    std::vector<Value> stack;
    std::unordered_map<std::string, Value> variables;
    std::vector<Value> locals;
    std::vector<HeapObject> heap;
    std::unordered_map<std::string, std::size_t> symbol_table;
    std::size_t pc = 0;
    bool halted = false;
    std::vector<std::string> output;

    // Bound on nested non-tail closure calls. Tail calls loop in
    // `execute_closure` and don't grow the native stack, but a plain
    // CallFunction recurses; an adversarial deep (non-tail) recursion would
    // otherwise overflow the C++ stack. This caps native depth and throws.
    std::size_t call_depth = 0;
    // Kept well below the point where the (large) native frames here overflow
    // the C++ stack; deep recursion is expected to use tail calls, which loop.
    static constexpr std::size_t kMaxCallDepth = 256;

    // Execute `code` from the start until HALT or the end.
    void execute(const CodeObject& code) {
        pc = 0;
        halted = false;
        while (!halted && pc < code.instructions.size())
            execute_instruction(code.instructions[pc], code);
    }

    // Format a value the way `print` does.
    std::string format_value(const Value& v) const {
        std::vector<std::size_t> visited;
        return format_impl(v, visited);
    }

   private:
    Value pop() {
        if (stack.empty()) throw VmError("VmError: Stack underflow");
        Value v = std::move(stack.back());
        stack.pop_back();
        return v;
    }
    std::size_t allocate(HeapObject obj) {
        heap.push_back(std::move(obj));
        return heap.size() - 1;
    }
    bool is_valid(std::size_t addr) const { return addr < heap.size(); }
    std::size_t intern_symbol(const std::string& name) {
        auto it = symbol_table.find(name);
        if (it != symbol_table.end()) return it->second;
        std::size_t addr = allocate(HeapSymbol{name});
        symbol_table[name] = addr;
        return addr;
    }

    static bool is_int(const Value& v) { return v.kind == ValueKind::Integer; }

    void binary_int(LispOp op) {
        Value b = pop(), a = pop();
        if (!is_int(a) || !is_int(b))
            throw VmError("VmError: expected two integers");
        if (op == LispOp::Div && b.integer == 0)
            throw VmError("VmError: Division by zero");
        std::int64_t r = 0;
        switch (op) {
            case LispOp::Add: r = a.integer + b.integer; break;
            case LispOp::Sub: r = a.integer - b.integer; break;
            case LispOp::Mul: r = a.integer * b.integer; break;
            case LispOp::Div: r = a.integer / b.integer; break;
            case LispOp::CmpLt: r = a.integer < b.integer ? 1 : 0; break;
            case LispOp::CmpGt: r = a.integer > b.integer ? 1 : 0; break;
            default: break;
        }
        stack.push_back(Value::integer_(r));
    }

    void execute_instruction(const Instruction& instr, const CodeObject& code) {
        std::size_t idx = instr.operand.value_or(0);
        switch (instr.opcode) {
            case LispOp::LoadConst:
                if (idx >= code.constants.size())
                    throw VmError("VmError: Constant index out of bounds");
                stack.push_back(code.constants[idx]);
                ++pc;
                break;
            case LispOp::Pop: pop(); ++pc; break;
            case LispOp::LoadNil: stack.push_back(Value::nil()); ++pc; break;
            case LispOp::LoadTrue:
                stack.push_back(Value::boolean_(true));
                ++pc;
                break;
            case LispOp::StoreName: {
                if (idx >= code.names.size())
                    throw VmError("VmError: Name index out of bounds");
                variables[code.names[idx]] = pop();
                ++pc;
                break;
            }
            case LispOp::LoadName: {
                if (idx >= code.names.size())
                    throw VmError("VmError: Name index out of bounds");
                auto it = variables.find(code.names[idx]);
                if (it == variables.end())
                    throw VmError("VmError: Undefined variable: " +
                                  code.names[idx]);
                stack.push_back(it->second);
                ++pc;
                break;
            }
            case LispOp::StoreLocal: {
                Value v = pop();
                while (locals.size() <= idx) locals.push_back(Value::nil());
                locals[idx] = std::move(v);
                ++pc;
                break;
            }
            case LispOp::LoadLocal:
                stack.push_back(idx < locals.size() ? locals[idx]
                                                    : Value::nil());
                ++pc;
                break;
            case LispOp::Add:
            case LispOp::Sub:
            case LispOp::Mul:
            case LispOp::Div:
            case LispOp::CmpLt:
            case LispOp::CmpGt: binary_int(instr.opcode); ++pc; break;
            case LispOp::CmpEq: {
                Value b = pop(), a = pop();
                int r;
                if (a.kind == ValueKind::Nil && b.kind == ValueKind::Nil)
                    r = 1;
                else if (a.kind == ValueKind::Nil || b.kind == ValueKind::Nil)
                    r = 0;
                else
                    r = (a == b) ? 1 : 0;
                stack.push_back(Value::integer_(r));
                ++pc;
                break;
            }
            case LispOp::Jump: pc = idx; break;
            case LispOp::JumpIfFalse: {
                Value v = pop();
                pc = v.is_falsy() ? idx : pc + 1;
                break;
            }
            case LispOp::JumpIfTrue: {
                Value v = pop();
                pc = !v.is_falsy() ? idx : pc + 1;
                break;
            }
            case LispOp::MakeClosure: {
                std::size_t param_count = idx;
                Value code_val = pop();
                if (code_val.kind != ValueKind::Code || !code_val.code)
                    throw VmError("VmError: MAKE_CLOSURE expected CodeObject");
                CodeObject fc = *code_val.code;
                std::vector<std::string> params;
                for (std::size_t i = 0; i < param_count; ++i) {
                    std::string p;
                    if (fc.constants.size() >= param_count) {
                        const Value& cv =
                            fc.constants[fc.constants.size() - param_count + i];
                        if (cv.kind == ValueKind::String) p = cv.str;
                    }
                    if (p.empty()) p = "_p" + std::to_string(i);
                    params.push_back(p);
                }
                std::size_t addr = allocate(
                    LispClosure{std::move(fc), variables, std::move(params)});
                stack.push_back(Value::closure_addr(addr));
                ++pc;
                break;
            }
            case LispOp::CallFunction:
            case LispOp::TailCall: {
                std::size_t argc = idx;
                Value func = pop();
                std::vector<Value> args(argc);
                for (std::size_t i = 0; i < argc; ++i) args[argc - 1 - i] = pop();
                if (func.kind != ValueKind::ClosureAddr)
                    throw VmError("VmError: cannot call non-closure");
                execute_closure(func.addr, std::move(args));
                break;
            }
            case LispOp::Return: ++pc; break;  // handled by execute_closure
            case LispOp::Cons: {
                Value car = pop(), cdr = pop();
                std::size_t addr =
                    allocate(ConsCell{std::move(car), std::move(cdr)});
                stack.push_back(Value::cons_addr(addr));
                ++pc;
                break;
            }
            case LispOp::Car:
            case LispOp::Cdr: {
                Value av = pop();
                if (av.kind == ValueKind::ConsAddr && is_valid(av.addr)) {
                    if (const auto* c = std::get_if<ConsCell>(&heap[av.addr])) {
                        stack.push_back(instr.opcode == LispOp::Car ? c->car
                                                                    : c->cdr);
                        ++pc;
                        break;
                    }
                }
                throw VmError("VmError: not a cons cell");
            }
            case LispOp::MakeSymbol: {
                if (idx >= code.constants.size() ||
                    code.constants[idx].kind != ValueKind::String)
                    throw VmError("VmError: MAKE_SYMBOL constant is not a string");
                stack.push_back(
                    Value::cons_addr(intern_symbol(code.constants[idx].str)));
                ++pc;
                break;
            }
            case LispOp::IsAtom: {
                Value v = pop();
                int r = 1;
                if (v.kind == ValueKind::ConsAddr && is_valid(v.addr) &&
                    std::holds_alternative<ConsCell>(heap[v.addr]))
                    r = 0;
                stack.push_back(Value::integer_(r));
                ++pc;
                break;
            }
            case LispOp::IsNil: {
                Value v = pop();
                stack.push_back(
                    Value::integer_(v.kind == ValueKind::Nil ? 1 : 0));
                ++pc;
                break;
            }
            case LispOp::Print: {
                Value v = pop();
                output.push_back(format_value(v));
                ++pc;
                break;
            }
            case LispOp::Halt: halted = true; break;
        }
    }

    void execute_closure(std::size_t closure_addr, std::vector<Value> args) {
        if (!is_valid(closure_addr) ||
            !std::holds_alternative<LispClosure>(heap[closure_addr]))
            throw VmError("VmError: not a closure");
        if (call_depth >= kMaxCallDepth)
            throw VmError("VmError: call stack exhausted");
        // RAII: decrement on every exit, including exception unwinding.
        struct DepthGuard {
            std::size_t& d;
            explicit DepthGuard(std::size_t& c) : d(c) { ++d; }
            ~DepthGuard() { --d; }
        } depth_guard(call_depth);

        std::size_t saved_pc = pc;
        bool saved_halted = halted;
        auto saved_vars = variables;
        auto saved_locals = locals;

        {
            const auto& cl = std::get<LispClosure>(heap[closure_addr]);
            for (const auto& [k, v] : cl.env) variables[k] = v;
        }

        std::size_t cur_addr = closure_addr;
        std::vector<Value> cur_args = std::move(args);
        Value return_value = Value::nil();

        for (;;) {
            // Snapshot the closure's body/params (heap may grow during the run).
            CodeObject body;
            std::vector<std::string> params;
            {
                const auto& cl = std::get<LispClosure>(heap[cur_addr]);
                body = cl.code;
                params = cl.params;
            }
            locals = std::move(cur_args);
            pc = 0;
            halted = false;
            for (std::size_t i = 0; i < params.size(); ++i)
                if (i < locals.size()) variables[params[i]] = locals[i];

            bool did_tail = false;
            std::size_t tail_addr = 0;
            std::vector<Value> tail_args;

            while (!halted && pc < body.instructions.size()) {
                const Instruction& ins = body.instructions[pc];
                if (ins.opcode == LispOp::Return) {
                    return_value = stack.empty() ? Value::nil() : pop();
                    break;
                }
                if (ins.opcode == LispOp::Halt) break;
                if (ins.opcode == LispOp::TailCall) {
                    std::size_t argc = ins.operand.value_or(0);
                    Value func = pop();
                    std::vector<Value> na(argc);
                    for (std::size_t i = 0; i < argc; ++i)
                        na[argc - 1 - i] = pop();
                    if (func.kind != ValueKind::ClosureAddr ||
                        !is_valid(func.addr) ||
                        !std::holds_alternative<LispClosure>(heap[func.addr]))
                        throw VmError("VmError: cannot tail-call non-closure");
                    did_tail = true;
                    tail_addr = func.addr;
                    tail_args = std::move(na);
                    break;
                }
                execute_instruction(ins, body);
            }

            if (did_tail) {
                variables = saved_vars;
                const auto& nc = std::get<LispClosure>(heap[tail_addr]);
                for (const auto& [k, v] : nc.env) variables[k] = v;
                cur_addr = tail_addr;
                cur_args = std::move(tail_args);
                continue;
            }
            break;
        }

        pc = saved_pc;
        halted = saved_halted;
        variables = std::move(saved_vars);
        locals = std::move(saved_locals);
        stack.push_back(std::move(return_value));
        ++pc;
    }

    std::string format_impl(const Value& v,
                            std::vector<std::size_t>& visited) const {
        char buf[64];
        switch (v.kind) {
            case ValueKind::Nil: return "nil";
            case ValueKind::Bool: return v.boolean ? "t" : "nil";
            case ValueKind::Integer: return std::to_string(v.integer);
            case ValueKind::String:
            case ValueKind::Symbol: return v.str;
            case ValueKind::ConsAddr:
                if (is_valid(v.addr)) {
                    const HeapObject& o = heap[v.addr];
                    if (std::holds_alternative<ConsCell>(o))
                        return format_cons(v.addr, visited);
                    if (const auto* s = std::get_if<HeapSymbol>(&o))
                        return s->name;
                    std::snprintf(buf, sizeof buf, "<closure @%zu>", v.addr);
                    return buf;
                }
                std::snprintf(buf, sizeof buf, "<invalid @%zu>", v.addr);
                return buf;
            case ValueKind::ClosureAddr:
                std::snprintf(buf, sizeof buf, "<closure @%zu>", v.addr);
                return buf;
            case ValueKind::Code: return "<code>";
        }
        return "";
    }

    std::string format_cons(std::size_t addr,
                            std::vector<std::size_t>& visited) const {
        std::string parts;
        bool first = true;
        std::size_t current = addr;
        for (;;) {
            for (std::size_t seen : visited)
                if (seen == current) {
                    if (!first) parts += " ";
                    parts += "...";
                    return "(" + parts + ")";
                }
            visited.push_back(current);
            if (!is_valid(current) ||
                !std::holds_alternative<ConsCell>(heap[current]))
                return "(" + parts + ")";
            const auto& cell = std::get<ConsCell>(heap[current]);
            if (!first) parts += " ";
            first = false;
            parts += format_impl(cell.car, visited);
            const Value& cdr = cell.cdr;
            if (cdr.kind == ValueKind::Nil) return "(" + parts + ")";
            if (cdr.kind == ValueKind::ConsAddr && is_valid(cdr.addr) &&
                std::holds_alternative<ConsCell>(heap[cdr.addr])) {
                current = cdr.addr;
                continue;
            }
            return "(" + parts + " . " + format_impl(cdr, visited) + ")";
        }
    }
};

// ── Top-level pipeline ───────────────────────────────────────────────────────

// Compile and execute `source`, returning the result (top of stack, or Nil).
inline Value run(const std::string& source) {
    CodeObject code = lc::compile(source);
    LispVm vm;
    vm.execute(code);
    return vm.stack.empty() ? Value::nil() : vm.stack.back();
}

// Like run() but also returns the (print ...) output.
inline std::pair<Value, std::vector<std::string>> run_with_output(
    const std::string& source) {
    CodeObject code = lc::compile(source);
    LispVm vm;
    vm.execute(code);
    Value result = vm.stack.empty() ? Value::nil() : vm.stack.back();
    return {result, vm.output};
}

}  // namespace ca::lisp_vm

#endif  // LISP_VM_HPP
