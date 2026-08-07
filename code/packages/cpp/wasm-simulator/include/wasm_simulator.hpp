// wasm_simulator.hpp — a stack-based WebAssembly virtual machine, in pure ISO
// C++17, header-only, in namespace ca::wasm. A faithful port of the Rust
// `wasm-simulator` crate.
// ===========================================================================
//
// WASM is a STACK machine: operands live on an implicit operand stack rather
// than in named registers. `i32.const 10 / i32.const 20 / i32.add` pushes 10 and
// 20, then `add` pops both and pushes 30. Bytecode is variable-length: one
// opcode byte, optionally followed by operand bytes.
//
// Supported opcodes (an i32 subset): 0x0B end, 0x20 local.get, 0x21 local.set,
// 0x41 i32.const, 0x6A i32.add, 0x6B i32.sub. Arithmetic wraps modulo 2^32.
//
// The simulator decodes and executes bytecode, producing a WasmStepTrace per
// instruction (stack before/after, a locals snapshot, and a description).
//
// DIVERGENCE. Where the Rust panics (unknown opcode / truncated code / stack
// underflow / stepping a halted VM), this port throws (std::runtime_error /
// std::out_of_range), the idiomatic C++ equivalent.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_WASM_SIMULATOR_HPP
#define CA_WASM_SIMULATOR_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace wasm {

// Supported opcodes.
inline constexpr std::uint8_t OP_END = 0x0B;
inline constexpr std::uint8_t OP_LOCAL_GET = 0x20;
inline constexpr std::uint8_t OP_LOCAL_SET = 0x21;
inline constexpr std::uint8_t OP_I32_CONST = 0x41;
inline constexpr std::uint8_t OP_I32_ADD = 0x6A;
inline constexpr std::uint8_t OP_I32_SUB = 0x6B;

// A decoded instruction.
struct WasmInstruction {
    std::uint8_t opcode;
    std::string mnemonic;
    std::optional<std::int32_t> operand;
    std::size_t size;
};

// A complete record of one instruction's execution.
struct WasmStepTrace {
    std::size_t pc;
    WasmInstruction instruction;
    std::vector<std::int32_t> stack_before;
    std::vector<std::int32_t> stack_after;
    std::vector<std::int32_t> locals_snapshot;
    std::string description;
    bool halted;
};

// Decodes variable-length bytecode into structured instructions.
class WasmDecoder {
public:
    WasmInstruction decode(const std::vector<std::uint8_t>& bytecode,
                           std::size_t pc) const {
        if (pc >= bytecode.size()) {
            throw std::out_of_range("WASM decode past end of bytecode");
        }
        std::uint8_t opcode = bytecode[pc];
        switch (opcode) {
            case OP_I32_CONST: {
                if (pc + 4 >= bytecode.size()) {
                    throw std::out_of_range("truncated i32.const operand");
                }
                std::uint32_t v = static_cast<std::uint32_t>(bytecode[pc + 1]) |
                                  (static_cast<std::uint32_t>(bytecode[pc + 2]) << 8) |
                                  (static_cast<std::uint32_t>(bytecode[pc + 3]) << 16) |
                                  (static_cast<std::uint32_t>(bytecode[pc + 4]) << 24);
                return {opcode, "i32.const", static_cast<std::int32_t>(v), 5};
            }
            case OP_I32_ADD:
                return {opcode, "i32.add", std::nullopt, 1};
            case OP_I32_SUB:
                return {opcode, "i32.sub", std::nullopt, 1};
            case OP_LOCAL_GET:
                if (pc + 1 >= bytecode.size()) {
                    throw std::out_of_range("truncated local.get operand");
                }
                return {opcode, "local.get",
                        static_cast<std::int32_t>(bytecode[pc + 1]), 2};
            case OP_LOCAL_SET:
                if (pc + 1 >= bytecode.size()) {
                    throw std::out_of_range("truncated local.set operand");
                }
                return {opcode, "local.set",
                        static_cast<std::int32_t>(bytecode[pc + 1]), 2};
            case OP_END:
                return {opcode, "end", std::nullopt, 1};
            default:
                throw std::runtime_error("Unknown WASM opcode: " +
                                         std::to_string(opcode));
        }
    }
};

// Executes instructions by mutating the stack and locals in place.
class WasmExecutor {
public:
    WasmStepTrace execute(const WasmInstruction& inst,
                          std::vector<std::int32_t>& stack,
                          std::vector<std::int32_t>& locals,
                          std::size_t pc) const {
        std::vector<std::int32_t> stack_before = stack;
        WasmStepTrace tr;
        tr.pc = pc;
        tr.instruction = inst;
        tr.stack_before = stack_before;
        tr.halted = false;

        if (inst.mnemonic == "i32.const") {
            std::int32_t val = *inst.operand;
            stack.push_back(val);
            tr.description = "push " + std::to_string(val);
        } else if (inst.mnemonic == "i32.add" || inst.mnemonic == "i32.sub") {
            std::int32_t b = pop(stack), a = pop(stack);
            std::uint32_t res_u =
                (inst.mnemonic == "i32.add")
                    ? static_cast<std::uint32_t>(a) + static_cast<std::uint32_t>(b)
                    : static_cast<std::uint32_t>(a) - static_cast<std::uint32_t>(b);
            std::int32_t res = static_cast<std::int32_t>(res_u);
            stack.push_back(res);
            tr.description = "pop " + std::to_string(b) + " and " +
                             std::to_string(a) + ", push " + std::to_string(res);
        } else if (inst.mnemonic == "local.get") {
            std::size_t idx = static_cast<std::size_t>(*inst.operand);
            std::int32_t val = locals.at(idx);
            stack.push_back(val);
            tr.description = "push locals[" + std::to_string(idx) + "] = " +
                             std::to_string(val);
        } else if (inst.mnemonic == "local.set") {
            std::size_t idx = static_cast<std::size_t>(*inst.operand);
            std::int32_t val = pop(stack);
            locals.at(idx) = val;
            tr.description = "pop " + std::to_string(val) + ", store in locals[" +
                             std::to_string(idx) + "]";
        } else if (inst.mnemonic == "end") {
            tr.description = "halt";
            tr.halted = true;
        } else {
            throw std::runtime_error("Cannot execute: " + inst.mnemonic);
        }

        tr.stack_after = stack;
        tr.locals_snapshot = locals;
        return tr;
    }

private:
    static std::int32_t pop(std::vector<std::int32_t>& stack) {
        if (stack.empty()) throw std::runtime_error("Stack underflow");
        std::int32_t v = stack.back();
        stack.pop_back();
        return v;
    }
};

// The full simulation environment.
class WasmSimulator {
public:
    std::vector<std::int32_t> stack;
    std::vector<std::int32_t> locals;
    std::size_t pc = 0;
    std::vector<std::uint8_t> bytecode;
    bool halted = false;
    std::size_t cycle = 0;

    explicit WasmSimulator(std::size_t num_locals) : locals(num_locals, 0) {}

    void load(const std::vector<std::uint8_t>& code) {
        bytecode = code;
        pc = 0;
        halted = false;
        cycle = 0;
        stack.clear();
        std::fill(locals.begin(), locals.end(), 0);
    }

    // Execute one instruction. Throws if the VM has already halted.
    WasmStepTrace step() {
        if (halted) {
            throw std::runtime_error(
                "WASM simulator has halted -- no more instructions");
        }
        WasmInstruction inst = decoder_.decode(bytecode, pc);
        WasmStepTrace tr = executor_.execute(inst, stack, locals, pc);
        pc += inst.size;
        halted = tr.halted;
        cycle++;
        return tr;
    }

    // Run a program to completion (an `end`) or until max_steps.
    std::vector<WasmStepTrace> run(const std::vector<std::uint8_t>& program,
                                   std::size_t max_steps) {
        load(program);
        std::vector<WasmStepTrace> traces;
        for (std::size_t i = 0; i < max_steps; i++) {
            if (halted) break;
            traces.push_back(step());
        }
        return traces;
    }

private:
    WasmDecoder decoder_;
    WasmExecutor executor_;
};

// ── Encoding helpers ─────────────────────────────────────────────────────────

inline std::vector<std::uint8_t> encode_i32_const(std::int32_t val) {
    std::uint32_t v = static_cast<std::uint32_t>(val);
    return {OP_I32_CONST, static_cast<std::uint8_t>(v),
            static_cast<std::uint8_t>(v >> 8), static_cast<std::uint8_t>(v >> 16),
            static_cast<std::uint8_t>(v >> 24)};
}
inline std::vector<std::uint8_t> encode_i32_add() { return {OP_I32_ADD}; }
inline std::vector<std::uint8_t> encode_i32_sub() { return {OP_I32_SUB}; }
inline std::vector<std::uint8_t> encode_local_get(std::uint8_t idx) {
    return {OP_LOCAL_GET, idx};
}
inline std::vector<std::uint8_t> encode_local_set(std::uint8_t idx) {
    return {OP_LOCAL_SET, idx};
}
inline std::vector<std::uint8_t> encode_end() { return {OP_END}; }

// Assemble a sequence of encoded instructions into flat bytecode.
inline std::vector<std::uint8_t> assemble_wasm(
    const std::vector<std::vector<std::uint8_t>>& instructions) {
    std::vector<std::uint8_t> out;
    for (const auto& ins : instructions)
        out.insert(out.end(), ins.begin(), ins.end());
    return out;
}

}  // namespace wasm
}  // namespace ca

#endif  // CA_WASM_SIMULATOR_HPP
