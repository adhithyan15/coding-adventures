// jvm_simulator.hpp — a typed stack-based JVM virtual machine, in pure ISO
// C++17, header-only, in namespace ca::jvm. A faithful port of the Rust
// `jvm-simulator` crate.
// ===========================================================================
//
// Like WASM, the JVM is a STACK machine — but a TYPED one: instead of a generic
// `add`, the operand type lives in the opcode (`iadd` / `ladd` / `fadd` /
// `dadd`), so a verifier can prove type safety at class-load time. Locals are
// numbered slots (this VM models the int subset); compact opcodes exist for
// slots 0-3 (`iload_0`, `istore_2`, ...).
//
// Supported opcodes (an int subset): iconst_0..5, bipush, ldc (constant pool),
// iload / iload_0..3, istore / istore_0..3, iadd / isub / imul / idiv,
// if_icmpeq / if_icmpgt / goto (16-bit signed branch offsets), ireturn /
// return. Arithmetic wraps modulo 2^32.
//
// The simulator decodes and executes bytecode, producing a JVMTrace per
// instruction (the stack before/after, a locals snapshot — each slot is a
// std::optional so it may be UNINITIALIZED — and a description).
//
// DIVERGENCE. Where the Rust panics (halted step, PC past the end, unknown or
// truncated opcode, stack underflow, a constant-pool index out of range,
// division by zero, an uninitialized / out-of-range local), this port throws
// (std::runtime_error / std::out_of_range), the idiomatic C++ equivalent.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under
// GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_JVM_SIMULATOR_HPP
#define CA_JVM_SIMULATOR_HPP

#include <cstdint>
#include <cstdio>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace jvm {

// ── Opcodes (the supported int subset) ───────────────────────────────────────
inline constexpr std::uint8_t OP_ICONST_0 = 0x03;
inline constexpr std::uint8_t OP_ICONST_5 = 0x08;
inline constexpr std::uint8_t OP_BIPUSH = 0x10;
inline constexpr std::uint8_t OP_LDC = 0x12;
inline constexpr std::uint8_t OP_ILOAD = 0x15;
inline constexpr std::uint8_t OP_ILOAD_0 = 0x1A;
inline constexpr std::uint8_t OP_ILOAD_3 = 0x1D;
inline constexpr std::uint8_t OP_ISTORE = 0x36;
inline constexpr std::uint8_t OP_ISTORE_0 = 0x3B;
inline constexpr std::uint8_t OP_ISTORE_3 = 0x3E;
inline constexpr std::uint8_t OP_IADD = 0x60;
inline constexpr std::uint8_t OP_ISUB = 0x64;
inline constexpr std::uint8_t OP_IMUL = 0x68;
inline constexpr std::uint8_t OP_IDIV = 0x6C;
inline constexpr std::uint8_t OP_IF_ICMPEQ = 0x9F;
inline constexpr std::uint8_t OP_IF_ICMPGT = 0xA3;
inline constexpr std::uint8_t OP_GOTO = 0xA7;
inline constexpr std::uint8_t OP_IRETURN = 0xAC;
inline constexpr std::uint8_t OP_RETURN = 0xB1;

// A record of one instruction's execution. `locals_snapshot[i]` is empty when
// slot i is uninitialized.
struct JVMTrace {
    std::size_t pc;
    std::string opcode;
    std::vector<std::int32_t> stack_before;
    std::vector<std::int32_t> stack_after;
    std::vector<std::optional<std::int32_t>> locals_snapshot;
    std::string description;
};

// The JVM simulator — a typed stack-based virtual machine.
class JVMSimulator {
public:
    std::vector<std::int32_t> stack;
    std::vector<std::optional<std::int32_t>> locals;
    std::vector<std::int32_t> constants;
    std::size_t pc = 0;
    bool halted = false;
    std::optional<std::int32_t> return_value;

    JVMSimulator() : locals(16) {}  // 16 uninitialized locals by default

    // Load bytecode, a constant pool, and a locals count; resets all state.
    void load(const std::vector<std::uint8_t>& bc,
              const std::vector<std::int32_t>& consts, std::size_t num_locals) {
        bytecode_ = bc;
        constants = consts;
        stack.clear();
        locals.assign(num_locals, std::nullopt);
        pc = 0;
        halted = false;
        return_value = std::nullopt;
    }

    // Execute one instruction and return its trace. Throws if halted or if PC is
    // past the end of the bytecode.
    JVMTrace step() {
        if (halted) throw std::runtime_error("JVM simulator has halted");
        if (pc >= bytecode_.size())
            throw std::out_of_range("PC past end of bytecode");
        std::size_t at = pc;
        std::vector<std::int32_t> stack_before = stack;
        return execute(bytecode_[at], stack_before, at);
    }

    // Run until halt or `max_steps`.
    std::vector<JVMTrace> run(std::size_t max_steps) {
        std::vector<JVMTrace> traces;
        for (std::size_t i = 0; i < max_steps; i++) {
            if (halted) break;
            traces.push_back(step());
        }
        return traces;
    }

private:
    std::vector<std::uint8_t> bytecode_;

    JVMTrace execute(std::uint8_t opcode,
                     const std::vector<std::int32_t>& stack_before,
                     std::size_t at) {
        // iconst_N: push a small constant 0-5.
        if (opcode >= OP_ICONST_0 && opcode <= OP_ICONST_5) {
            std::int32_t val = static_cast<std::int32_t>(opcode - OP_ICONST_0);
            stack.push_back(val);
            pc += 1;
            return trace(at, "iconst_" + std::to_string(val), stack_before,
                         "push " + std::to_string(val));
        }
        // iload_N / istore_N: compact local access for slots 0-3.
        if (opcode >= OP_ILOAD_0 && opcode <= OP_ILOAD_3) {
            std::size_t slot = static_cast<std::size_t>(opcode - OP_ILOAD_0);
            return do_iload(at, slot, "iload_" + std::to_string(slot),
                            stack_before, 1);
        }
        if (opcode >= OP_ISTORE_0 && opcode <= OP_ISTORE_3) {
            std::size_t slot = static_cast<std::size_t>(opcode - OP_ISTORE_0);
            return do_istore(at, slot, "istore_" + std::to_string(slot),
                             stack_before, 1);
        }

        switch (opcode) {
            case OP_BIPUSH: {
                std::int32_t val = static_cast<std::int8_t>(operand_byte(at, 1));
                stack.push_back(val);
                pc += 2;
                return trace(at, "bipush", stack_before,
                             "push " + std::to_string(val));
            }
            case OP_LDC: {
                std::size_t idx = operand_byte(at, 1);
                if (idx >= constants.size())
                    throw std::out_of_range("Constant pool index " +
                                            std::to_string(idx) +
                                            " out of range");
                std::int32_t val = constants[idx];
                stack.push_back(val);
                pc += 2;
                return trace(at, "ldc", stack_before,
                             "push constant[" + std::to_string(idx) + "] = " +
                                 std::to_string(val));
            }
            case OP_ILOAD:
                return do_iload(at, operand_byte(at, 1), "iload", stack_before,
                                2);
            case OP_ISTORE:
                return do_istore(at, operand_byte(at, 1), "istore", stack_before,
                                 2);
            case OP_IADD:
                return do_binary(at, "iadd", stack_before,
                                 [](std::int32_t a, std::int32_t b) {
                                     return static_cast<std::int32_t>(
                                         static_cast<std::uint32_t>(a) +
                                         static_cast<std::uint32_t>(b));
                                 });
            case OP_ISUB:
                return do_binary(at, "isub", stack_before,
                                 [](std::int32_t a, std::int32_t b) {
                                     return static_cast<std::int32_t>(
                                         static_cast<std::uint32_t>(a) -
                                         static_cast<std::uint32_t>(b));
                                 });
            case OP_IMUL:
                return do_binary(at, "imul", stack_before,
                                 [](std::int32_t a, std::int32_t b) {
                                     return static_cast<std::int32_t>(
                                         static_cast<std::uint32_t>(a) *
                                         static_cast<std::uint32_t>(b));
                                 });
            case OP_IDIV: {
                if (stack.size() < 2) throw std::runtime_error("Stack underflow");
                if (stack.back() == 0)
                    throw std::runtime_error(
                        "ArithmeticException: division by zero");
                return do_binary(at, "idiv", stack_before,
                                 [](std::int32_t a, std::int32_t b) {
                                     // wrapping_div: only overflow is MIN / -1.
                                     if (a == INT32_MIN && b == -1)
                                         return INT32_MIN;
                                     return a / b;
                                 });
            }
            case OP_GOTO: {
                std::size_t target = branch_target(at);
                pc = target;
                return trace(at, "goto", stack_before,
                             "jump to PC=" + std::to_string(target));
            }
            case OP_IF_ICMPEQ:
                return do_if_icmp(at, "if_icmpeq", stack_before,
                                  [](std::int32_t a, std::int32_t b) {
                                      return a == b;
                                  });
            case OP_IF_ICMPGT:
                return do_if_icmp(at, "if_icmpgt", stack_before,
                                  [](std::int32_t a, std::int32_t b) {
                                      return a > b;
                                  });
            case OP_IRETURN: {
                if (stack.empty()) throw std::runtime_error("Stack underflow");
                std::int32_t val = stack.back();
                stack.pop_back();
                return_value = val;
                halted = true;
                pc += 1;
                return trace(at, "ireturn", stack_before,
                             "return " + std::to_string(val));
            }
            case OP_RETURN:
                halted = true;
                pc += 1;
                return trace(at, "return", stack_before, "return void");
            default: {
                char buf[32];
                std::snprintf(buf, sizeof buf, "0x%02X", opcode);
                throw std::runtime_error(std::string("Unimplemented opcode: ") +
                                         buf);
            }
        }
    }

    // ---- shared instruction bodies -----------------------------------------

    JVMTrace do_iload(std::size_t at, std::size_t slot, const std::string& mn,
                      const std::vector<std::int32_t>& stack_before,
                      std::size_t size) {
        if (slot >= locals.size())
            throw std::out_of_range("Local slot out of range");
        if (!locals[slot].has_value())
            throw std::runtime_error("Local variable uninitialized");
        std::int32_t val = *locals[slot];
        stack.push_back(val);
        pc += size;
        return trace(at, mn, stack_before,
                     "push locals[" + std::to_string(slot) + "] = " +
                         std::to_string(val));
    }

    JVMTrace do_istore(std::size_t at, std::size_t slot, const std::string& mn,
                       const std::vector<std::int32_t>& stack_before,
                       std::size_t size) {
        if (slot >= locals.size())
            throw std::out_of_range("Local slot out of range");
        if (stack.empty()) throw std::runtime_error("Stack underflow");
        std::int32_t val = stack.back();
        stack.pop_back();
        locals[slot] = val;
        pc += size;
        return trace(at, mn, stack_before,
                     "pop " + std::to_string(val) + ", store in locals[" +
                         std::to_string(slot) + "]");
    }

    template <typename F>
    JVMTrace do_binary(std::size_t at, const std::string& mn,
                       const std::vector<std::int32_t>& stack_before, F op) {
        if (stack.size() < 2) throw std::runtime_error("Stack underflow");
        std::int32_t b = stack.back();
        stack.pop_back();
        std::int32_t a = stack.back();
        stack.pop_back();
        std::int32_t result = op(a, b);
        stack.push_back(result);
        pc += 1;
        return trace(at, mn, stack_before,
                     "pop " + std::to_string(b) + " and " + std::to_string(a) +
                         ", push " + std::to_string(result));
    }

    template <typename F>
    JVMTrace do_if_icmp(std::size_t at, const std::string& mn,
                        const std::vector<std::int32_t>& stack_before, F op) {
        if (stack.size() < 2) throw std::runtime_error("Stack underflow");
        std::int32_t offset = branch_offset(at);
        std::int32_t b = stack.back();
        stack.pop_back();
        std::int32_t a = stack.back();
        stack.pop_back();
        std::string desc;
        if (op(a, b)) {
            std::size_t target = static_cast<std::size_t>(
                static_cast<std::int64_t>(at) + offset);
            pc = target;
            desc = "pop " + std::to_string(b) + " and " + std::to_string(a) +
                   ", true, jump to PC=" + std::to_string(target);
        } else {
            pc = at + 3;
            desc = "pop " + std::to_string(b) + " and " + std::to_string(a) +
                   ", false, fall through";
        }
        return trace(at, mn, stack_before, desc);
    }

    // ---- decoding helpers ---------------------------------------------------

    std::uint8_t operand_byte(std::size_t at, std::size_t k) const {
        if (at + k >= bytecode_.size())
            throw std::out_of_range("truncated operand");
        return bytecode_[at + k];
    }

    std::int32_t branch_offset(std::size_t at) const {
        std::uint16_t raw =
            static_cast<std::uint16_t>(operand_byte(at, 1) << 8) |
            operand_byte(at, 2);
        return (raw >= 0x8000) ? static_cast<std::int32_t>(raw) - 0x10000
                               : static_cast<std::int32_t>(raw);
    }

    std::size_t branch_target(std::size_t at) const {
        return static_cast<std::size_t>(static_cast<std::int64_t>(at) +
                                        branch_offset(at));
    }

    JVMTrace trace(std::size_t at, const std::string& opcode,
                   const std::vector<std::int32_t>& stack_before,
                   const std::string& description) const {
        JVMTrace t;
        t.pc = at;
        t.opcode = opcode;
        t.stack_before = stack_before;
        t.stack_after = stack;
        t.locals_snapshot = locals;
        t.description = description;
        return t;
    }
};

// ── Encoding helpers ─────────────────────────────────────────────────────────

// Encode a small integer constant: iconst_N for 0-5, bipush for -128..127.
inline std::vector<std::uint8_t> encode_iconst(std::int32_t n) {
    if (n >= 0 && n <= 5)
        return {static_cast<std::uint8_t>(OP_ICONST_0 + n)};
    if (n >= -128 && n <= 127)
        return {OP_BIPUSH, static_cast<std::uint8_t>(n)};
    throw std::out_of_range("Out of range for encode_iconst");
}

// Encode istore / iload, using the compact form for slots 0-3.
inline std::vector<std::uint8_t> encode_istore(std::uint8_t slot) {
    if (slot <= 3) return {static_cast<std::uint8_t>(OP_ISTORE_0 + slot)};
    return {OP_ISTORE, slot};
}
inline std::vector<std::uint8_t> encode_iload(std::uint8_t slot) {
    if (slot <= 3) return {static_cast<std::uint8_t>(OP_ILOAD_0 + slot)};
    return {OP_ILOAD, slot};
}

// A bytecode instruction for the assembler.
struct Instr {
    std::uint8_t opcode;
    std::vector<std::int32_t> params;
};

// Assemble a sequence of instructions into flat bytecode (1-byte operand for
// bipush/iload/istore/ldc, 2-byte big-endian offset for goto/if_icmp*).
inline std::vector<std::uint8_t> assemble_jvm(const std::vector<Instr>& instrs) {
    std::vector<std::uint8_t> out;
    for (const auto& in : instrs) {
        out.push_back(in.opcode);
        switch (in.opcode) {
            case OP_BIPUSH:
            case OP_ILOAD:
            case OP_ISTORE:
            case OP_LDC:
                out.push_back(static_cast<std::uint8_t>(in.params.at(0)));
                break;
            case OP_GOTO:
            case OP_IF_ICMPEQ:
            case OP_IF_ICMPGT: {
                std::uint16_t off =
                    static_cast<std::uint16_t>(in.params.at(0));
                out.push_back(static_cast<std::uint8_t>((off >> 8) & 0xFF));
                out.push_back(static_cast<std::uint8_t>(off & 0xFF));
                break;
            }
            default:
                break;
        }
    }
    return out;
}

}  // namespace jvm
}  // namespace ca

#endif  // CA_JVM_SIMULATOR_HPP
