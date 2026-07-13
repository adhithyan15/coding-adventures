// intel8008_simulator.hpp — Intel 8008 behavioral simulator, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `intel8008-simulator` crate, in namespace
// `ca::intel8008_simulator`: a behavioral simulator for the Intel 8008 (1972),
// the world's first 8-bit microprocessor.
//
// It executes 8008 machine code directly (no gate-level modelling): registers
// A/B/C/D/E/H/L, the M pseudo-register (memory at [H:L]), four condition flags
// (carry / zero / sign / parity), a 16 KiB address space, and the 8008's unique
// 8-level push-down call stack (entry[0] IS the program counter).
//
// `step()` executes one instruction and returns a `Trace`; `run()` loads a
// program at address 0 and steps until HLT, an error, or `max_steps`. Where the
// Rust `step()` returns `Result`, this port throws `std::runtime_error`. Pure
// ISO C++17.

#ifndef INTEL8008_SIMULATOR_HPP
#define INTEL8008_SIMULATOR_HPP

#include <array>
#include <cstdarg>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace intel8008_simulator {

// ── Condition flags ──────────────────────────────────────────────────────────
struct Flags {
    bool carry = false;
    bool zero = false;
    bool sign = false;
    bool parity = false;
    bool operator==(const Flags& o) const {
        return carry == o.carry && zero == o.zero && sign == o.sign &&
               parity == o.parity;
    }
};

// ── One instruction-execution record ─────────────────────────────────────────
struct Trace {
    std::uint16_t address = 0;
    std::vector<std::uint8_t> raw;
    std::string mnemonic;
    std::uint8_t a_before = 0;
    std::uint8_t a_after = 0;
    Flags flags_before;
    Flags flags_after;
    std::optional<std::uint16_t> mem_address;
    std::optional<std::uint8_t> mem_value;
};

namespace detail {
// 3-bit register-field names and ALU mnemonics (for disassembly).
constexpr const char* kRegNames[8] = {"B", "C", "D", "E", "H", "L", "M", "A"};
constexpr const char* kAluMnem[8] = {"ADD", "ADC", "SUB", "SBB",
                                     "ANA", "XRA", "ORA", "CMP"};
constexpr const char* kAluImmMnem[8] = {"ADI", "ACI", "SUI", "SBI",
                                        "ANI", "XRI", "ORI", "CPI"};
inline std::string fmt(const char* pattern, ...) {
    char buf[32];
    va_list ap;
    va_start(ap, pattern);
    std::vsnprintf(buf, sizeof buf, pattern, ap);
    va_end(ap);
    return std::string(buf);
}
}  // namespace detail

// ── The simulator ────────────────────────────────────────────────────────────
class Simulator {
  public:
    Simulator() { memory_.assign(16384, 0); }

    // ── Accessors ───────────────────────────────────────────────────────────
    std::uint8_t a() const { return regs_[7]; }
    std::uint8_t b() const { return regs_[0]; }
    std::uint8_t c() const { return regs_[1]; }
    std::uint8_t d() const { return regs_[2]; }
    std::uint8_t e() const { return regs_[3]; }
    std::uint8_t h() const { return regs_[4]; }
    std::uint8_t l() const { return regs_[5]; }
    std::uint16_t pc() const { return stack_[0] & 0x3FFF; }
    std::uint16_t hl_address() const {
        return static_cast<std::uint16_t>(
            ((static_cast<std::uint16_t>(regs_[4]) & 0x3F) << 8) | regs_[5]);
    }
    Flags flags() const { return flags_; }
    std::size_t stack_depth() const { return stack_depth_; }
    bool halted() const { return halted_; }

    // ── I/O ports ───────────────────────────────────────────────────────────
    void set_input_port(std::size_t port, std::uint8_t value) {
        if (port < 8) {
            input_ports_[port] = value;
        }
    }
    std::uint8_t get_output_port(std::size_t port) const {
        return port < 24 ? output_ports_[port] : 0;
    }

    // ── Program load / reset ────────────────────────────────────────────────
    void load_program(const std::vector<std::uint8_t>& program,
                      std::size_t start) {
        std::size_t end = start + program.size();
        if (end > 16384) {
            end = 16384;
        }
        for (std::size_t i = start; i < end; ++i) {
            memory_[i] = program[i - start];
        }
    }
    void reset() {
        regs_.fill(0);
        stack_.fill(0);
        stack_depth_ = 0;
        flags_ = Flags{};
        halted_ = false;
    }

    // ── Step / run ──────────────────────────────────────────────────────────
    Trace step();  // throws on halt / unknown opcode

    std::vector<Trace> run(const std::vector<std::uint8_t>& program,
                           std::size_t max_steps) {
        reset();
        load_program(program, 0);
        std::vector<Trace> traces;
        for (std::size_t i = 0; i < max_steps; ++i) {
            try {
                Trace t = step();
                bool h = halted_;
                traces.push_back(std::move(t));
                if (h) {
                    break;
                }
            } catch (const std::runtime_error&) {
                break;
            }
        }
        return traces;
    }

  private:
    std::uint8_t mem_read(std::uint16_t addr) const {
        return memory_[addr & 0x3FFF];
    }
    void mem_write(std::uint16_t addr, std::uint8_t v) {
        memory_[addr & 0x3FFF] = v;
    }
    std::uint8_t reg_read(std::size_t idx) const {
        return idx == 6 ? mem_read(hl_address()) : regs_[idx];
    }
    void push_and_jump(std::uint16_t target) {
        for (int i = 7; i >= 1; --i) {
            stack_[i] = stack_[i - 1];
        }
        stack_[0] = target & 0x3FFF;
        if (stack_depth_ < 7) {
            ++stack_depth_;
        }
    }
    void pop_return() {
        for (int i = 0; i < 7; ++i) {
            stack_[i] = stack_[i + 1];
        }
        stack_[7] = 0;
        if (stack_depth_ > 0) {
            --stack_depth_;
        }
    }
    static Flags compute_flags(std::uint8_t result, bool carry,
                               bool update_carry, Flags prev) {
        int ones = 0;
        for (int i = 0; i < 8; ++i) {
            ones += (result >> i) & 1;
        }
        Flags f;
        f.carry = update_carry ? carry : prev.carry;
        f.zero = result == 0;
        f.sign = (result & 0x80) != 0;
        f.parity = (ones % 2) == 0;
        return f;
    }
    bool condition_met(std::uint8_t ccc, bool sense) const {
        bool v = false;
        switch (ccc & 0x03) {
            case 0: v = flags_.carry; break;
            case 1: v = flags_.zero; break;
            case 2: v = flags_.sign; break;
            case 3: v = flags_.parity; break;
        }
        return sense ? v : !v;
    }
    // (result, carry_out, clear_carry)
    static void alu_op(std::uint8_t alu, std::uint8_t a, std::uint8_t b,
                       bool carry_in, std::uint8_t& res, bool& carry,
                       bool& clear_carry) {
        std::uint16_t wide;
        clear_carry = false;
        switch (alu) {
            case 0:
                wide = static_cast<std::uint16_t>(a) + b;
                res = static_cast<std::uint8_t>(wide);
                carry = wide > 0xFF;
                break;
            case 1: {
                std::uint16_t ci = carry_in ? 1 : 0;
                wide = static_cast<std::uint16_t>(a) + b + ci;
                res = static_cast<std::uint8_t>(wide);
                carry = wide > 0xFF;
                break;
            }
            case 2:
                res = static_cast<std::uint8_t>(a - b);
                carry = a < b;
                break;
            case 3: {
                unsigned bi = carry_in ? 1 : 0;
                unsigned total = static_cast<unsigned>(b) + bi;
                res = static_cast<std::uint8_t>(a - total);
                carry = static_cast<unsigned>(a) < total;
                break;
            }
            case 4: res = a & b; carry = false; clear_carry = true; break;
            case 5: res = a ^ b; carry = false; clear_carry = true; break;
            case 6: res = a | b; carry = false; clear_carry = true; break;
            default:  // 7 CMP
                res = static_cast<std::uint8_t>(a - b);
                carry = a < b;
                break;
        }
    }
    std::uint8_t fetch_byte() {
        std::uint16_t pc = stack_[0] & 0x3FFF;
        std::uint8_t byte = memory_[pc];
        stack_[0] = static_cast<std::uint16_t>((pc + 1) & 0x3FFF);
        return byte;
    }
    static std::string cond_suffix(std::uint8_t ccc, bool sense) {
        const char* letter;
        switch (ccc & 0x03) {
            case 0: letter = "C"; break;
            case 1: letter = "Z"; break;
            case 2: letter = "S"; break;
            default: letter = "P"; break;
        }
        return std::string(sense ? "T" : "F") + letter;
    }

    std::array<std::uint8_t, 8> regs_{};
    std::vector<std::uint8_t> memory_;
    std::array<std::uint16_t, 8> stack_{};
    std::size_t stack_depth_ = 0;
    Flags flags_;
    bool halted_ = false;
    std::array<std::uint8_t, 8> input_ports_{};
    std::array<std::uint8_t, 24> output_ports_{};
};

inline Trace Simulator::step() {
    using detail::fmt;
    using detail::kAluImmMnem;
    using detail::kAluMnem;
    using detail::kRegNames;
    if (halted_) {
        throw std::runtime_error("CPU is halted");
    }
    std::uint16_t fetch_pc = stack_[0] & 0x3FFF;
    std::uint8_t a_before = regs_[7];
    Flags flags_before = flags_;

    std::uint8_t opcode = fetch_byte();
    Trace tr;
    tr.raw.push_back(opcode);
    std::uint8_t group = (opcode >> 6) & 0x03;
    std::uint8_t ddd = (opcode >> 3) & 0x07;
    std::uint8_t sss = opcode & 0x07;
    std::string mnemonic;

    if (group == 0) {
        switch (sss) {
            case 0: {  // INR DDD
                std::uint8_t result =
                    static_cast<std::uint8_t>(reg_read(ddd) + 1);
                if (ddd == 6) {
                    std::uint16_t addr = hl_address();
                    mem_write(addr, result);
                    tr.mem_address = addr;
                    tr.mem_value = result;
                } else {
                    regs_[ddd] = result;
                }
                flags_ = compute_flags(result, false, false, flags_);
                mnemonic = fmt("INR %.1s", kRegNames[ddd]);
                break;
            }
            case 1: {  // DCR DDD
                std::uint8_t result =
                    static_cast<std::uint8_t>(reg_read(ddd) - 1);
                if (ddd == 6) {
                    std::uint16_t addr = hl_address();
                    mem_write(addr, result);
                    tr.mem_address = addr;
                    tr.mem_value = result;
                } else {
                    regs_[ddd] = result;
                }
                flags_ = compute_flags(result, false, false, flags_);
                mnemonic = fmt("DCR %.1s", kRegNames[ddd]);
                break;
            }
            case 2: {  // rotates (ddd 0-3) or OUT (ddd 4-7)
                std::uint8_t r;
                bool cy;
                switch (ddd) {
                    case 0:
                        cy = (regs_[7] >> 7) & 1;
                        r = static_cast<std::uint8_t>((regs_[7] << 1) |
                                                      ((regs_[7] >> 7) & 1));
                        regs_[7] = r;
                        flags_.carry = cy;
                        mnemonic = "RLC";
                        break;
                    case 1:
                        cy = regs_[7] & 1;
                        r = static_cast<std::uint8_t>((regs_[7] >> 1) |
                                                      ((regs_[7] & 1) << 7));
                        regs_[7] = r;
                        flags_.carry = cy;
                        mnemonic = "RRC";
                        break;
                    case 2:
                        cy = (regs_[7] >> 7) & 1;
                        r = static_cast<std::uint8_t>((regs_[7] << 1) |
                                                      (flags_.carry ? 1 : 0));
                        regs_[7] = r;
                        flags_.carry = cy;
                        mnemonic = "RAL";
                        break;
                    case 3:
                        cy = regs_[7] & 1;
                        r = static_cast<std::uint8_t>(
                            (flags_.carry ? 0x80 : 0) | (regs_[7] >> 1));
                        regs_[7] = r;
                        flags_.carry = cy;
                        mnemonic = "RAR";
                        break;
                    default: {  // OUT
                        std::size_t port = (opcode >> 1) & 0x1F;
                        if (port < 24) {
                            output_ports_[port] = regs_[7];
                        }
                        mnemonic = fmt("OUT %zu", port);
                        break;
                    }
                }
                break;
            }
            case 3: {  // return-if-false
                std::uint8_t ccc = ddd & 0x03;
                mnemonic = "R" + cond_suffix(ccc, false);
                if (condition_met(ccc, false)) {
                    pop_return();
                }
                break;
            }
            case 5: {  // RST
                std::uint16_t target = static_cast<std::uint16_t>(ddd << 3);
                mnemonic = fmt("RST %u", (unsigned)ddd);
                push_and_jump(target);
                break;
            }
            case 6: {  // MVI DDD, data
                std::uint8_t data = fetch_byte();
                tr.raw.push_back(data);
                if (ddd == 6) {
                    std::uint16_t addr = hl_address();
                    mem_write(addr, data);
                    tr.mem_address = addr;
                    tr.mem_value = data;
                    mnemonic = fmt("MVI M, 0x%02X", data);
                } else {
                    regs_[ddd] = data;
                    mnemonic = fmt("MVI %.1s, 0x%02X", kRegNames[ddd], data);
                }
                break;
            }
            case 7: {  // return-if-true (0x3F = unconditional RET)
                if (opcode == 0x3F) {
                    pop_return();
                    mnemonic = "RET";
                } else {
                    std::uint8_t ccc = ddd & 0x03;
                    mnemonic = "R" + cond_suffix(ccc, true);
                    if (condition_met(ccc, true)) {
                        pop_return();
                    }
                }
                break;
            }
            default:
                throw std::runtime_error("unknown opcode");
        }
    } else if (group == 1) {
        if (opcode == 0x76) {
            halted_ = true;
            mnemonic = "HLT";
        } else if (opcode == 0x7C) {  // JMP
            std::uint8_t lo = fetch_byte();
            std::uint8_t hi = fetch_byte();
            tr.raw.push_back(lo);
            tr.raw.push_back(hi);
            std::uint16_t target = static_cast<std::uint16_t>(
                ((static_cast<std::uint16_t>(hi) & 0x3F) << 8) | lo);
            stack_[0] = target;
            mnemonic = fmt("JMP 0x%04X", target);
        } else if (opcode == 0x7E) {  // CAL
            std::uint8_t lo = fetch_byte();
            std::uint8_t hi = fetch_byte();
            tr.raw.push_back(lo);
            tr.raw.push_back(hi);
            std::uint16_t target = static_cast<std::uint16_t>(
                ((static_cast<std::uint16_t>(hi) & 0x3F) << 8) | lo);
            push_and_jump(target);
            mnemonic = fmt("CAL 0x%04X", target);
        } else if (sss == 1) {  // IN
            std::size_t port = ddd;
            regs_[7] = input_ports_[port < 7 ? port : 7];
            mnemonic = fmt("IN %zu", port);
        } else if ((sss == 0 || sss == 4) && ddd <= 3) {  // conditional jump
            std::uint8_t lo = fetch_byte();
            std::uint8_t hi = fetch_byte();
            tr.raw.push_back(lo);
            tr.raw.push_back(hi);
            std::uint16_t target = static_cast<std::uint16_t>(
                ((static_cast<std::uint16_t>(hi) & 0x3F) << 8) | lo);
            bool sense = sss == 4;
            mnemonic = "J" + cond_suffix(ddd, sense) + fmt(" 0x%04X", target);
            if (condition_met(ddd, sense)) {
                stack_[0] = target;
            }
        } else if ((sss == 2 || sss == 6) && ddd <= 3) {  // conditional call
            std::uint8_t lo = fetch_byte();
            std::uint8_t hi = fetch_byte();
            tr.raw.push_back(lo);
            tr.raw.push_back(hi);
            std::uint16_t target = static_cast<std::uint16_t>(
                ((static_cast<std::uint16_t>(hi) & 0x3F) << 8) | lo);
            bool sense = sss == 6;
            mnemonic = "C" + cond_suffix(ddd, sense) + fmt(" 0x%04X", target);
            if (condition_met(ddd, sense)) {
                push_and_jump(target);
            }
        } else {  // MOV DDD, SSS
            std::uint8_t src_val = reg_read(sss);
            if (sss == 6) {
                tr.mem_address = hl_address();
                tr.mem_value = src_val;
            }
            if (ddd == 6) {
                std::uint16_t addr = hl_address();
                mem_write(addr, src_val);
                tr.mem_address = addr;
                tr.mem_value = src_val;
            } else {
                regs_[ddd] = src_val;
            }
            mnemonic = fmt("MOV %.1s, %.1s", kRegNames[ddd], kRegNames[sss]);
        }
    } else if (group == 2) {  // ALU register
        std::uint8_t src_val = reg_read(sss);
        if (sss == 6) {
            tr.mem_address = hl_address();
            tr.mem_value = src_val;
        }
        std::uint8_t result;
        bool carry, clear_carry;
        alu_op(ddd, regs_[7], src_val, flags_.carry, result, carry,
               clear_carry);
        flags_ = compute_flags(result, clear_carry ? false : carry, true,
                               flags_);
        if (ddd != 7) {
            regs_[7] = result;
        }
        mnemonic = fmt("%.3s %.1s", kAluMnem[ddd], kRegNames[sss]);
    } else {  // group == 3: ALU immediate / HLT
        if (opcode == 0xFF) {
            halted_ = true;
            mnemonic = "HLT";
        } else if (sss == 4) {
            std::uint8_t data = fetch_byte();
            tr.raw.push_back(data);
            std::uint8_t result;
            bool carry, clear_carry;
            alu_op(ddd, regs_[7], data, flags_.carry, result, carry,
                   clear_carry);
            flags_ = compute_flags(result, clear_carry ? false : carry, true,
                                   flags_);
            if (ddd != 7) {
                regs_[7] = result;
            }
            mnemonic = fmt("%.3s 0x%02X", kAluImmMnem[ddd], data);
        } else {
            throw std::runtime_error("unknown opcode");
        }
    }

    tr.address = fetch_pc;
    tr.mnemonic = std::move(mnemonic);
    tr.a_before = a_before;
    tr.a_after = regs_[7];
    tr.flags_before = flags_before;
    tr.flags_after = flags_;
    return tr;
}

}  // namespace intel8008_simulator
}  // namespace ca

#endif  // INTEL8008_SIMULATOR_HPP
