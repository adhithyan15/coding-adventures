// intel4004_simulator.hpp — Intel 4004 behavioral simulator, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `intel4004-simulator` crate, in namespace
// `ca::intel4004_simulator`: a behavioral simulator for the Intel 4004 (1971),
// the world's first commercial single-chip microprocessor (2,300 transistors).
//
// The 4004 is natively 4-bit: every value is a nibble (0-15) and arithmetic is
// masked to 4 bits. It is an accumulator machine — operations funnel through a
// single Accumulator. Memory is a byte-addressable ROM plus a small data RAM
// (4 banks × 4 registers × 16 characters), RAM status nibbles, a 3-deep
// hardware call stack, and I/O ports.
//
// Instructions are 1 or 2 bytes; the upper nibble of the first byte is the
// opcode. `step()` executes one instruction and returns a `Trace`; `run()`
// resets, loads a program at address 0, and steps until HLT or `max_steps`.
// Where the Rust `step()` asserts `!halted`, this port throws
// `std::runtime_error`. Pure ISO C++17.

#ifndef INTEL4004_SIMULATOR_HPP
#define INTEL4004_SIMULATOR_HPP

#include <algorithm>
#include <array>
#include <cstdarg>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace intel4004_simulator {

// ── One instruction-execution record ─────────────────────────────────────────
// The accumulator and carry are captured before and after. For a two-byte
// instruction `raw2` holds the second byte.
struct Trace {
    std::size_t address = 0;
    std::uint8_t raw = 0;
    std::optional<std::uint8_t> raw2;
    std::string mnemonic;
    std::uint8_t accumulator_before = 0;
    std::uint8_t accumulator_after = 0;
    bool carry_before = false;
    bool carry_after = false;
};

namespace detail {
inline std::string fmt(const char* pattern, ...) {
    char buf[24];
    std::va_list ap;
    va_start(ap, pattern);
    std::vsnprintf(buf, sizeof buf, pattern, ap);
    va_end(ap);
    return std::string(buf);
}

// Two-byte first-byte predicate: upper nibble 0x1/0x4/0x5/0x7, or 0x2 even
// (FIM; the odd 0x2 form is single-byte SRC).
inline bool is_two_byte(std::uint8_t raw) {
    std::uint8_t upper = static_cast<std::uint8_t>((raw >> 4) & 0xF);
    if (upper == 0x1 || upper == 0x4 || upper == 0x5 || upper == 0x7) {
        return true;
    }
    return upper == 0x2 && (raw & 0x1) == 0;
}
}  // namespace detail

// ── The simulator ────────────────────────────────────────────────────────────
class Simulator {
  public:
    // Construct with `memory_size` bytes of ROM (typically 4096).
    explicit Simulator(std::size_t memory_size = 4096)
        : memory_(memory_size, 0), memory_size_(memory_size) {}

    // Reset all CPU state to zero. Called by run() before each program.
    void reset() {
        accumulator_ = 0;
        registers_.fill(0);
        carry_ = false;
        std::fill(memory_.begin(), memory_.end(), std::uint8_t{0});
        pc_ = 0;
        halted_ = false;
        hw_stack_.fill(0);
        stack_pointer_ = 0;
        for (auto& bank : ram_) {
            for (auto& reg : bank) {
                reg.fill(0);
            }
        }
        for (auto& bank : ram_status_) {
            for (auto& reg : bank) {
                reg.fill(0);
            }
        }
        ram_output_.fill(0);
        ram_bank_ = 0;
        ram_register_ = 0;
        ram_character_ = 0;
        rom_port_ = 0;
    }

    // Copy `program` to ROM at address 0 (clamped), reset PC, clear halted.
    void load_program(const std::vector<std::uint8_t>& program) {
        std::size_t n = program.size();
        if (n > memory_size_) {
            n = memory_size_;
        }
        for (std::size_t i = 0; i < n; ++i) {
            memory_[i] = program[i];
        }
        pc_ = 0;
        halted_ = false;
    }

    // Execute one instruction and return its trace. Throws std::runtime_error
    // if the CPU has already halted (the Rust `step()` asserts `!halted`).
    Trace step() {
        if (halted_) {
            throw std::runtime_error("CPU is halted");
        }

        Trace t;
        t.address = pc_;
        std::uint8_t raw = rom_read(pc_);
        pc_ += 1;
        t.raw = raw;

        if (detail::is_two_byte(raw)) {
            t.raw2 = rom_read(pc_);
            pc_ += 1;
        }

        t.accumulator_before = accumulator_;
        t.carry_before = carry_;

        std::uint8_t opcode = static_cast<std::uint8_t>((raw >> 4) & 0xF);
        std::uint8_t operand = static_cast<std::uint8_t>(raw & 0xF);
        t.mnemonic = execute(opcode, operand, raw, t.raw2, t.address);

        t.accumulator_after = accumulator_;
        t.carry_after = carry_;
        return t;
    }

    // Reset, load `program`, and step up to `max_steps` (stopping at HLT).
    // Returns the trace of every executed instruction.
    std::vector<Trace> run(const std::vector<std::uint8_t>& program,
                           std::size_t max_steps) {
        reset();
        load_program(program);
        std::vector<Trace> traces;
        for (std::size_t i = 0; i < max_steps; ++i) {
            if (halted_) {
                break;
            }
            traces.push_back(step());
        }
        return traces;
    }

    // ── State accessors ──────────────────────────────────────────────────────
    std::uint8_t accumulator() const { return accumulator_; }
    bool carry() const { return carry_; }
    std::uint8_t register_at(std::size_t r) const {
        return r < registers_.size() ? registers_[r] : 0;
    }
    std::size_t pc() const { return pc_; }
    bool halted() const { return halted_; }
    std::uint16_t hw_stack(std::size_t i) const {
        return i < hw_stack_.size() ? hw_stack_[i] : 0;
    }
    std::size_t stack_pointer() const { return stack_pointer_; }
    std::uint8_t ram(std::size_t bank, std::size_t reg, std::size_t chr) const {
        if (bank < 4 && reg < 4 && chr < 16) {
            return ram_[bank][reg][chr];
        }
        return 0;
    }
    std::uint8_t ram_status(std::size_t bank, std::size_t reg,
                            std::size_t idx) const {
        if (bank < 4 && reg < 4 && idx < 4) {
            return ram_status_[bank][reg][idx];
        }
        return 0;
    }
    std::uint8_t ram_output(std::size_t bank) const {
        return bank < 4 ? ram_output_[bank] : 0;
    }
    std::size_t ram_bank() const { return ram_bank_; }
    std::size_t ram_register() const { return ram_register_; }
    std::size_t ram_character() const { return ram_character_; }
    std::uint8_t rom_port() const { return rom_port_; }

  private:
    // Bounds-checked ROM read: past the end reads as 0x00 (NOP), so a runaway
    // PC stays safe rather than reading out of bounds.
    std::uint8_t rom_read(std::size_t addr) const {
        return addr < memory_size_ ? memory_[addr] : 0;
    }

    // Register pairs: pair p groups (R[2p], R[2p+1]) as high:low nibbles.
    std::uint8_t read_pair(std::size_t p) const {
        std::uint8_t hi = static_cast<std::uint8_t>(registers_[p * 2] & 0xF);
        std::uint8_t lo = static_cast<std::uint8_t>(registers_[p * 2 + 1] & 0xF);
        return static_cast<std::uint8_t>((hi << 4) | lo);
    }
    void write_pair(std::size_t p, std::uint8_t val) {
        registers_[p * 2] = static_cast<std::uint8_t>((val >> 4) & 0xF);
        registers_[p * 2 + 1] = static_cast<std::uint8_t>(val & 0xF);
    }

    // Hardware stack: 3 deep, wraps silently on the 4th push.
    void stack_push(std::uint16_t addr) {
        hw_stack_[stack_pointer_ % 3] = static_cast<std::uint16_t>(addr & 0xFFF);
        stack_pointer_ = (stack_pointer_ + 1) % 3;
    }
    std::uint16_t stack_pop() {
        stack_pointer_ = stack_pointer_ == 0 ? 2 : stack_pointer_ - 1;
        return hw_stack_[stack_pointer_ % 3];
    }

    // Currently addressed RAM nibbles (bank via DCL, register/char via SRC).
    std::uint8_t ram_read_main() const {
        return static_cast<std::uint8_t>(
            ram_[ram_bank_][ram_register_][ram_character_] & 0xF);
    }
    void ram_write_main(std::uint8_t val) {
        ram_[ram_bank_][ram_register_][ram_character_] =
            static_cast<std::uint8_t>(val & 0xF);
    }
    std::uint8_t ram_read_status(std::size_t idx) const {
        return static_cast<std::uint8_t>(
            ram_status_[ram_bank_][ram_register_][idx] & 0xF);
    }
    void ram_write_status(std::size_t idx, std::uint8_t val) {
        ram_status_[ram_bank_][ram_register_][idx] =
            static_cast<std::uint8_t>(val & 0xF);
    }

    // Execute the decoded instruction, returning its disassembly. Mirrors the
    // Rust `execute` dispatch exactly.
    std::string execute(std::uint8_t opcode, std::uint8_t operand,
                        std::uint8_t raw, std::optional<std::uint8_t> raw2,
                        std::size_t address) {
        if (raw == 0x00) {
            return "NOP";
        }
        if (raw == 0x01) {
            halted_ = true;
            return "HLT";
        }

        switch (opcode) {
        case 0x1: {  // JCN — jump conditional
            std::uint8_t cond = operand;
            std::uint8_t addr_low = raw2.value_or(0);
            bool invert = (cond & 0x8) != 0;
            bool test_zero = (cond & 0x4) != 0;
            bool test_carry = (cond & 0x2) != 0;
            bool test_pin = (cond & 0x1) != 0;
            bool result = false;
            if (test_zero) {
                result = result || (accumulator_ == 0);
            }
            if (test_carry) {
                result = result || carry_;
            }
            if (test_pin) {
                result = result || false;  // test pin always low in the sim
            }
            if (invert) {
                result = !result;
            }
            if (result) {
                std::size_t page = (address + 2) & 0xF00;
                pc_ = page | static_cast<std::size_t>(addr_low);
            }
            return detail::fmt("JCN 0x%X,0x%02X", cond, addr_low);
        }
        case 0x2:
            if ((raw & 1) == 0) {  // FIM — fetch immediate to register pair
                std::size_t pair = static_cast<std::size_t>(operand >> 1);
                write_pair(pair, raw2.value_or(0));
                return detail::fmt("FIM P%u,0x%02X",
                                   static_cast<unsigned>(pair),
                                   raw2.value_or(0));
            } else {  // SRC — send register control
                std::size_t pair = static_cast<std::size_t>(operand >> 1);
                std::uint8_t pair_val = read_pair(pair);
                ram_register_ = static_cast<std::size_t>((pair_val >> 4) & 0x3);
                ram_character_ = static_cast<std::size_t>(pair_val & 0xF);
                return detail::fmt("SRC P%u", static_cast<unsigned>(pair));
            }
        case 0x3:
            if ((raw & 1) == 0) {  // FIN — fetch indirect from ROM
                std::size_t pair = static_cast<std::size_t>(operand >> 1);
                std::size_t rom_addr = static_cast<std::size_t>(read_pair(0));
                std::size_t page = pc_ & 0xF00;
                write_pair(pair, rom_read(page | rom_addr));
                return detail::fmt("FIN P%u", static_cast<unsigned>(pair));
            } else {  // JIN — jump indirect through register pair
                std::size_t pair = static_cast<std::size_t>(operand >> 1);
                std::size_t pair_val = static_cast<std::size_t>(read_pair(pair));
                std::size_t page = pc_ & 0xF00;
                pc_ = page | pair_val;
                return detail::fmt("JIN P%u", static_cast<unsigned>(pair));
            }
        case 0x4: {  // JUN — jump unconditional (12-bit)
            std::size_t target =
                (static_cast<std::size_t>(operand) << 8) | raw2.value_or(0);
            pc_ = target;
            return detail::fmt("JUN 0x%03X", static_cast<unsigned>(target));
        }
        case 0x5: {  // JMS — jump to subroutine
            std::size_t target =
                (static_cast<std::size_t>(operand) << 8) | raw2.value_or(0);
            stack_push(static_cast<std::uint16_t>(pc_));
            pc_ = target;
            return detail::fmt("JMS 0x%03X", static_cast<unsigned>(target));
        }
        case 0x6: {  // INC — increment register (no carry effect)
            std::size_t reg = static_cast<std::size_t>(operand);
            registers_[reg] = static_cast<std::uint8_t>((registers_[reg] + 1) &
                                                        0xF);
            return detail::fmt("INC R%u", static_cast<unsigned>(reg));
        }
        case 0x7: {  // ISZ — increment and skip if zero
            std::size_t reg = static_cast<std::size_t>(operand);
            std::uint8_t addr_low = raw2.value_or(0);
            registers_[reg] = static_cast<std::uint8_t>((registers_[reg] + 1) &
                                                        0xF);
            if (registers_[reg] != 0) {
                std::size_t page = (address + 2) & 0xF00;
                pc_ = page | static_cast<std::size_t>(addr_low);
            }
            return detail::fmt("ISZ R%u,0x%02X", static_cast<unsigned>(reg),
                               addr_low);
        }
        case 0x8: {  // ADD — accumulator + register + carry
            std::size_t reg = static_cast<std::size_t>(operand & 0xF);
            unsigned carry_in = carry_ ? 1u : 0u;
            unsigned result = static_cast<unsigned>(accumulator_) +
                              registers_[reg] + carry_in;
            carry_ = result > 0xF;
            accumulator_ = static_cast<std::uint8_t>(result & 0xF);
            return detail::fmt("ADD R%u", static_cast<unsigned>(reg));
        }
        case 0x9: {  // SUB — complement-and-add (inverted carry)
            std::size_t reg = static_cast<std::size_t>(operand & 0xF);
            unsigned complement =
                static_cast<unsigned>((~registers_[reg]) & 0xF);
            unsigned borrow_in = carry_ ? 0u : 1u;
            unsigned result =
                static_cast<unsigned>(accumulator_) + complement + borrow_in;
            carry_ = result > 0xF;
            accumulator_ = static_cast<std::uint8_t>(result & 0xF);
            return detail::fmt("SUB R%u", static_cast<unsigned>(reg));
        }
        case 0xA: {  // LD — load register into accumulator
            std::size_t reg = static_cast<std::size_t>(operand & 0xF);
            accumulator_ = static_cast<std::uint8_t>(registers_[reg] & 0xF);
            return detail::fmt("LD R%u", static_cast<unsigned>(reg));
        }
        case 0xB: {  // XCH — exchange accumulator and register
            std::size_t reg = static_cast<std::size_t>(operand & 0xF);
            std::uint8_t old_a = accumulator_;
            accumulator_ = static_cast<std::uint8_t>(registers_[reg] & 0xF);
            registers_[reg] = static_cast<std::uint8_t>(old_a & 0xF);
            return detail::fmt("XCH R%u", static_cast<unsigned>(reg));
        }
        case 0xC: {  // BBL — branch back and load (subroutine return)
            std::uint16_t ret_addr = stack_pop();
            pc_ = static_cast<std::size_t>(ret_addr);
            accumulator_ = static_cast<std::uint8_t>(operand & 0xF);
            return detail::fmt("BBL %u", static_cast<unsigned>(operand));
        }
        case 0xD:  // LDM — load immediate into accumulator
            accumulator_ = static_cast<std::uint8_t>(operand & 0xF);
            return detail::fmt("LDM %u", static_cast<unsigned>(operand));
        case 0xE:
            return execute_io(raw);
        case 0xF:
            return execute_accumulator_group(raw);
        default:
            return detail::fmt("UNKNOWN(0x%02X)", raw);
        }
    }

    // 0xE group: RAM main/status, RAM output port, ROM port, ADM/SBM.
    std::string execute_io(std::uint8_t raw) {
        switch (raw) {
        case 0xE0:
            ram_write_main(accumulator_);
            return "WRM";
        case 0xE1:
            ram_output_[ram_bank_] = static_cast<std::uint8_t>(accumulator_ &
                                                               0xF);
            return "WMP";
        case 0xE2:
            rom_port_ = static_cast<std::uint8_t>(accumulator_ & 0xF);
            return "WRR";
        case 0xE3:
            return "WPM";  // write program memory: no-op in the simulator
        case 0xE4:
            ram_write_status(0, accumulator_);
            return "WR0";
        case 0xE5:
            ram_write_status(1, accumulator_);
            return "WR1";
        case 0xE6:
            ram_write_status(2, accumulator_);
            return "WR2";
        case 0xE7:
            ram_write_status(3, accumulator_);
            return "WR3";
        case 0xE8: {  // SBM — subtract RAM from accumulator
            std::uint8_t mem_val = ram_read_main();
            unsigned complement = static_cast<unsigned>((~mem_val) & 0xF);
            unsigned borrow_in = carry_ ? 0u : 1u;
            unsigned result =
                static_cast<unsigned>(accumulator_) + complement + borrow_in;
            carry_ = result > 0xF;
            accumulator_ = static_cast<std::uint8_t>(result & 0xF);
            return "SBM";
        }
        case 0xE9:
            accumulator_ = ram_read_main();
            return "RDM";
        case 0xEA:
            accumulator_ = static_cast<std::uint8_t>(rom_port_ & 0xF);
            return "RDR";
        case 0xEB: {  // ADM — add RAM to accumulator with carry
            std::uint8_t mem_val = ram_read_main();
            unsigned carry_in = carry_ ? 1u : 0u;
            unsigned result =
                static_cast<unsigned>(accumulator_) + mem_val + carry_in;
            carry_ = result > 0xF;
            accumulator_ = static_cast<std::uint8_t>(result & 0xF);
            return "ADM";
        }
        case 0xEC:
            accumulator_ = ram_read_status(0);
            return "RD0";
        case 0xED:
            accumulator_ = ram_read_status(1);
            return "RD1";
        case 0xEE:
            accumulator_ = ram_read_status(2);
            return "RD2";
        case 0xEF:
            accumulator_ = ram_read_status(3);
            return "RD3";
        default:
            return detail::fmt("UNKNOWN(0x%02X)", raw);
        }
    }

    // 0xF group: accumulator/carry manipulation, rotates, BCD, KBP, DCL.
    std::string execute_accumulator_group(std::uint8_t raw) {
        switch (raw) {
        case 0xF0:
            accumulator_ = 0;
            carry_ = false;
            return "CLB";
        case 0xF1:
            carry_ = false;
            return "CLC";
        case 0xF2: {  // IAC — increment accumulator
            unsigned result = static_cast<unsigned>(accumulator_) + 1;
            carry_ = result > 0xF;
            accumulator_ = static_cast<std::uint8_t>(result & 0xF);
            return "IAC";
        }
        case 0xF3:
            carry_ = !carry_;
            return "CMC";
        case 0xF4:
            accumulator_ = static_cast<std::uint8_t>((~accumulator_) & 0xF);
            return "CMA";
        case 0xF5: {  // RAL — rotate left through carry
            std::uint8_t old_carry = carry_ ? 1u : 0u;
            carry_ = (accumulator_ & 0x8) != 0;
            accumulator_ = static_cast<std::uint8_t>(
                ((accumulator_ << 1) | old_carry) & 0xF);
            return "RAL";
        }
        case 0xF6: {  // RAR — rotate right through carry
            std::uint8_t old_carry = carry_ ? 0x8u : 0u;
            carry_ = (accumulator_ & 0x1) != 0;
            accumulator_ = static_cast<std::uint8_t>(
                ((accumulator_ >> 1) | old_carry) & 0xF);
            return "RAR";
        }
        case 0xF7:  // TCC — transfer carry to accumulator, clear carry
            accumulator_ = static_cast<std::uint8_t>(carry_ ? 1 : 0);
            carry_ = false;
            return "TCC";
        case 0xF8:  // DAC — decrement accumulator (carry = no borrow)
            carry_ = accumulator_ > 0;
            accumulator_ = static_cast<std::uint8_t>((accumulator_ - 1) & 0xF);
            return "DAC";
        case 0xF9:  // TCS — transfer carry subtract, clear carry
            accumulator_ = static_cast<std::uint8_t>(carry_ ? 10 : 9);
            carry_ = false;
            return "TCS";
        case 0xFA:
            carry_ = true;
            return "STC";
        case 0xFB:  // DAA — decimal adjust accumulator
            if (accumulator_ > 9 || carry_) {
                unsigned result = static_cast<unsigned>(accumulator_) + 6;
                if (result > 0xF) {
                    carry_ = true;
                }
                accumulator_ = static_cast<std::uint8_t>(result & 0xF);
            }
            return "DAA";
        case 0xFC:  // KBP — one-hot keyboard decode
            switch (accumulator_) {
            case 0:
                accumulator_ = 0;
                break;
            case 1:
                accumulator_ = 1;
                break;
            case 2:
                accumulator_ = 2;
                break;
            case 4:
                accumulator_ = 3;
                break;
            case 8:
                accumulator_ = 4;
                break;
            default:
                accumulator_ = 15;
                break;
            }
            return "KBP";
        case 0xFD:  // DCL — designate command line (select RAM bank)
            ram_bank_ = static_cast<std::size_t>(accumulator_ & 0x7);
            if (ram_bank_ > 3) {
                ram_bank_ &= 3;
            }
            return "DCL";
        default:
            return detail::fmt("UNKNOWN(0x%02X)", raw);
        }
    }

    std::uint8_t accumulator_ = 0;
    std::array<std::uint8_t, 16> registers_{};
    bool carry_ = false;
    std::vector<std::uint8_t> memory_;
    std::size_t memory_size_ = 0;
    std::size_t pc_ = 0;
    bool halted_ = false;
    std::array<std::uint16_t, 3> hw_stack_{};
    std::size_t stack_pointer_ = 0;
    std::array<std::array<std::array<std::uint8_t, 16>, 4>, 4> ram_{};
    std::array<std::array<std::array<std::uint8_t, 4>, 4>, 4> ram_status_{};
    std::array<std::uint8_t, 4> ram_output_{};
    std::size_t ram_bank_ = 0;
    std::size_t ram_register_ = 0;
    std::size_t ram_character_ = 0;
    std::uint8_t rom_port_ = 0;
};

// ── Encoding helpers (free functions) ────────────────────────────────────────
inline std::uint8_t encode_nop() { return 0x00; }
inline std::uint8_t encode_hlt() { return 0x01; }
inline std::uint8_t encode_ldm(std::uint8_t n) {
    return static_cast<std::uint8_t>((0xD << 4) | (n & 0xF));
}
inline std::uint8_t encode_ld(std::uint8_t r) {
    return static_cast<std::uint8_t>((0xA << 4) | (r & 0xF));
}
inline std::uint8_t encode_xch(std::uint8_t r) {
    return static_cast<std::uint8_t>((0xB << 4) | (r & 0xF));
}
inline std::uint8_t encode_add(std::uint8_t r) {
    return static_cast<std::uint8_t>((0x8 << 4) | (r & 0xF));
}
inline std::uint8_t encode_sub(std::uint8_t r) {
    return static_cast<std::uint8_t>((0x9 << 4) | (r & 0xF));
}
inline std::uint8_t encode_inc(std::uint8_t r) {
    return static_cast<std::uint8_t>((0x6 << 4) | (r & 0xF));
}
inline std::uint8_t encode_bbl(std::uint8_t n) {
    return static_cast<std::uint8_t>((0xC << 4) | (n & 0xF));
}
inline std::uint8_t encode_src(std::uint8_t pair) {
    return static_cast<std::uint8_t>((0x2 << 4) | ((pair & 0x7) << 1) | 1);
}
inline std::uint8_t encode_fin(std::uint8_t pair) {
    return static_cast<std::uint8_t>((0x3 << 4) | ((pair & 0x7) << 1));
}
inline std::uint8_t encode_jin(std::uint8_t pair) {
    return static_cast<std::uint8_t>((0x3 << 4) | ((pair & 0x7) << 1) | 1);
}

// Two-byte encoders return {first, second}.
inline std::pair<std::uint8_t, std::uint8_t> encode_jcn(std::uint8_t cond,
                                                        std::uint8_t addr) {
    return {static_cast<std::uint8_t>((0x1 << 4) | (cond & 0xF)), addr};
}
inline std::pair<std::uint8_t, std::uint8_t> encode_fim(std::uint8_t pair,
                                                        std::uint8_t data) {
    return {static_cast<std::uint8_t>((0x2 << 4) | ((pair & 0x7) << 1)), data};
}
inline std::pair<std::uint8_t, std::uint8_t> encode_jun(std::uint16_t addr) {
    return {static_cast<std::uint8_t>((0x4 << 4) | ((addr >> 8) & 0xF)),
            static_cast<std::uint8_t>(addr & 0xFF)};
}
inline std::pair<std::uint8_t, std::uint8_t> encode_jms(std::uint16_t addr) {
    return {static_cast<std::uint8_t>((0x5 << 4) | ((addr >> 8) & 0xF)),
            static_cast<std::uint8_t>(addr & 0xFF)};
}
inline std::pair<std::uint8_t, std::uint8_t> encode_isz(std::uint8_t r,
                                                        std::uint8_t addr) {
    return {static_cast<std::uint8_t>((0x7 << 4) | (r & 0xF)), addr};
}

inline std::uint8_t encode_wrm() { return 0xE0; }
inline std::uint8_t encode_wmp() { return 0xE1; }
inline std::uint8_t encode_wrr() { return 0xE2; }
inline std::uint8_t encode_wpm() { return 0xE3; }
inline std::uint8_t encode_wr0() { return 0xE4; }
inline std::uint8_t encode_wr1() { return 0xE5; }
inline std::uint8_t encode_wr2() { return 0xE6; }
inline std::uint8_t encode_wr3() { return 0xE7; }
inline std::uint8_t encode_sbm() { return 0xE8; }
inline std::uint8_t encode_rdm() { return 0xE9; }
inline std::uint8_t encode_rdr() { return 0xEA; }
inline std::uint8_t encode_adm() { return 0xEB; }
inline std::uint8_t encode_rd0() { return 0xEC; }
inline std::uint8_t encode_rd1() { return 0xED; }
inline std::uint8_t encode_rd2() { return 0xEE; }
inline std::uint8_t encode_rd3() { return 0xEF; }

inline std::uint8_t encode_clb() { return 0xF0; }
inline std::uint8_t encode_clc() { return 0xF1; }
inline std::uint8_t encode_iac() { return 0xF2; }
inline std::uint8_t encode_cmc() { return 0xF3; }
inline std::uint8_t encode_cma() { return 0xF4; }
inline std::uint8_t encode_ral() { return 0xF5; }
inline std::uint8_t encode_rar() { return 0xF6; }
inline std::uint8_t encode_tcc() { return 0xF7; }
inline std::uint8_t encode_dac() { return 0xF8; }
inline std::uint8_t encode_tcs() { return 0xF9; }
inline std::uint8_t encode_stc() { return 0xFA; }
inline std::uint8_t encode_daa() { return 0xFB; }
inline std::uint8_t encode_kbp() { return 0xFC; }
inline std::uint8_t encode_dcl() { return 0xFD; }

}  // namespace intel4004_simulator
}  // namespace ca

#endif  // INTEL4004_SIMULATOR_HPP
