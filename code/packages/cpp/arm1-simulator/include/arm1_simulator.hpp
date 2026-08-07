// arm1_simulator.hpp — ARM1 behavioral CPU simulator, header-only C++17.
// ============================================================================
//
// A faithful port of the Rust `arm1-simulator` crate, in namespace
// `ca::arm1_simulator`: a complete behavioral simulator for the ARM1 (Sophie
// Wilson & Steve Furber, Acorn, 1985) — the first ARM chip. Implements the full
// ARMv1 instruction set: 16 data-processing ops, load/store, block transfer
// (LDM/STM), branch (B/BL), SWI, conditional execution, the inline barrel
// shifter, and 4 processor modes with banked registers.
//
// Pure ISO C++17.

#ifndef ARM1_SIMULATOR_HPP
#define ARM1_SIMULATOR_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

namespace ca {
namespace arm1_simulator {

// ── Constants ────────────────────────────────────────────────────────────────
constexpr std::uint32_t MODE_USR = 0, MODE_FIQ = 1, MODE_IRQ = 2, MODE_SVC = 3;

constexpr std::uint32_t COND_EQ = 0x0, COND_NE = 0x1, COND_CS = 0x2,
                        COND_CC = 0x3, COND_MI = 0x4, COND_PL = 0x5,
                        COND_VS = 0x6, COND_VC = 0x7, COND_HI = 0x8,
                        COND_LS = 0x9, COND_GE = 0xA, COND_LT = 0xB,
                        COND_GT = 0xC, COND_LE = 0xD, COND_AL = 0xE,
                        COND_NV = 0xF;

constexpr std::uint32_t OP_AND = 0x0, OP_EOR = 0x1, OP_SUB = 0x2, OP_RSB = 0x3,
                        OP_ADD = 0x4, OP_ADC = 0x5, OP_SBC = 0x6, OP_RSC = 0x7,
                        OP_TST = 0x8, OP_TEQ = 0x9, OP_CMP = 0xA, OP_CMN = 0xB,
                        OP_ORR = 0xC, OP_MOV = 0xD, OP_BIC = 0xE, OP_MVN = 0xF;

constexpr std::uint32_t SHIFT_LSL = 0, SHIFT_LSR = 1, SHIFT_ASR = 2,
                        SHIFT_ROR = 3;

constexpr std::uint32_t FLAG_N = 1u << 31, FLAG_Z = 1u << 30, FLAG_C = 1u << 29,
                        FLAG_V = 1u << 28, FLAG_I = 1u << 27, FLAG_F = 1u << 26;
constexpr std::uint32_t PC_MASK = 0x03FFFFFCu, MODE_MASK = 0x3u,
                        HALT_SWI = 0x123456u;

// ── Enum → string ────────────────────────────────────────────────────────────
inline std::string mode_string(std::uint32_t mode) {
    switch (mode) {
    case MODE_USR:
        return "USR";
    case MODE_FIQ:
        return "FIQ";
    case MODE_IRQ:
        return "IRQ";
    case MODE_SVC:
        return "SVC";
    default:
        return "???";
    }
}
inline std::string cond_string(std::uint32_t cond) {
    static const char* N[16] = {"EQ", "NE", "CS", "CC", "MI", "PL",
                                "VS", "VC", "HI", "LS", "GE", "LT",
                                "GT", "LE", "", "NV"};
    return cond < 16 ? N[cond] : "??";
}
inline std::string op_string(std::uint32_t opcode) {
    static const char* N[16] = {"AND", "EOR", "SUB", "RSB", "ADD", "ADC",
                                "SBC", "RSC", "TST", "TEQ", "CMP", "CMN",
                                "ORR", "MOV", "BIC", "MVN"};
    return opcode < 16 ? N[opcode] : "???";
}
inline bool is_test_op(std::uint32_t opcode) {
    return opcode >= OP_TST && opcode <= OP_CMN;
}
inline bool is_logical_op(std::uint32_t opcode) {
    return opcode == OP_AND || opcode == OP_EOR || opcode == OP_TST ||
           opcode == OP_TEQ || opcode == OP_ORR || opcode == OP_MOV ||
           opcode == OP_BIC || opcode == OP_MVN;
}
inline std::string shift_string(std::uint32_t shift_type) {
    switch (shift_type) {
    case SHIFT_LSL:
        return "LSL";
    case SHIFT_LSR:
        return "LSR";
    case SHIFT_ASR:
        return "ASR";
    case SHIFT_ROR:
        return "ROR";
    default:
        return "???";
    }
}

// ── Flags & condition evaluator ──────────────────────────────────────────────
struct Flags {
    bool n = false, z = false, c = false, v = false;
    bool operator==(const Flags& o) const {
        return n == o.n && z == o.z && c == o.c && v == o.v;
    }
};

inline bool evaluate_condition(std::uint32_t cond, Flags f) {
    switch (cond) {
    case COND_EQ:
        return f.z;
    case COND_NE:
        return !f.z;
    case COND_CS:
        return f.c;
    case COND_CC:
        return !f.c;
    case COND_MI:
        return f.n;
    case COND_PL:
        return !f.n;
    case COND_VS:
        return f.v;
    case COND_VC:
        return !f.v;
    case COND_HI:
        return f.c && !f.z;
    case COND_LS:
        return !f.c || f.z;
    case COND_GE:
        return f.n == f.v;
    case COND_LT:
        return f.n != f.v;
    case COND_GT:
        return !f.z && (f.n == f.v);
    case COND_LE:
        return f.z || (f.n != f.v);
    case COND_AL:
        return true;
    case COND_NV:
        return false;
    default:
        return false;
    }
}

// ── Barrel shifter ───────────────────────────────────────────────────────────
struct ShiftResult {
    std::uint32_t value;
    bool carry;
};

namespace detail {
inline std::uint32_t rotr32(std::uint32_t v, std::uint32_t amount) {
    amount &= 31;
    return amount == 0 ? v : (v >> amount) | (v << (32 - amount));
}
}  // namespace detail

inline ShiftResult barrel_shift(std::uint32_t value, std::uint32_t shift_type,
                                std::uint32_t amount, bool carry_in,
                                bool by_register) {
    if (by_register && amount == 0) {
        return {value, carry_in};
    }
    switch (shift_type) {
    case SHIFT_LSL: {
        if (amount == 0) return {value, carry_in};
        if (amount >= 32)
            return {0, amount == 32 ? (value & 1) != 0 : false};
        return {value << amount, ((value >> (32 - amount)) & 1) != 0};
    }
    case SHIFT_LSR: {
        if (amount == 0 && !by_register) return {0, (value >> 31) != 0};
        if (amount == 0) return {value, carry_in};
        if (amount >= 32)
            return {0, amount == 32 ? (value >> 31) != 0 : false};
        return {value >> amount, ((value >> (amount - 1)) & 1) != 0};
    }
    case SHIFT_ASR: {
        bool sign = (value >> 31) != 0;
        if (amount == 0 && !by_register)
            return sign ? ShiftResult{0xFFFFFFFFu, true} : ShiftResult{0, false};
        if (amount == 0) return {value, carry_in};
        if (amount >= 32)
            return sign ? ShiftResult{0xFFFFFFFFu, true} : ShiftResult{0, false};
        std::uint32_t r =
            static_cast<std::uint32_t>(static_cast<std::int32_t>(value) >>
                                       amount);
        return {r, ((value >> (amount - 1)) & 1) != 0};
    }
    case SHIFT_ROR: {
        if (amount == 0 && !by_register) {
            bool carry = (value & 1) != 0;
            std::uint32_t r = value >> 1;
            if (carry_in) r |= 0x80000000u;
            return {r, carry};
        }
        if (amount == 0) return {value, carry_in};
        amount &= 31;
        if (amount == 0) return {value, (value >> 31) != 0};
        std::uint32_t r = detail::rotr32(value, amount);
        return {r, ((r >> 31) & 1) != 0};
    }
    default:
        return {value, carry_in};
    }
}

inline ShiftResult decode_immediate(std::uint32_t imm8, std::uint32_t rotate) {
    std::uint32_t rotate_amount = rotate * 2;
    if (rotate_amount == 0) {
        return {imm8, false};
    }
    std::uint32_t value = detail::rotr32(imm8, rotate_amount);
    return {value, (value >> 31) != 0};
}

// ── ALU ──────────────────────────────────────────────────────────────────────
struct ALUResult {
    std::uint32_t result;
    bool n, z, c, v;
    bool write_result;
};

namespace detail {
struct AddResult {
    std::uint32_t result;
    bool carry, overflow;
};
inline AddResult add32(std::uint32_t a, std::uint32_t b, bool carry_in) {
    std::uint64_t sum = std::uint64_t(a) + std::uint64_t(b) + (carry_in ? 1 : 0);
    std::uint32_t result = static_cast<std::uint32_t>(sum);
    bool carry = (sum >> 32) != 0;
    bool overflow = ((((a ^ result) & (b ^ result)) >> 31) & 1) != 0;
    return {result, carry, overflow};
}
}  // namespace detail

inline ALUResult alu_execute(std::uint32_t opcode, std::uint32_t a,
                             std::uint32_t b, bool carry_in, bool shifter_carry,
                             bool old_v) {
    std::uint32_t result = 0;
    bool carry = false, overflow = false;
    auto logical = [&](std::uint32_t r) {
        result = r;
        carry = shifter_carry;
        overflow = old_v;
    };
    switch (opcode) {
    case OP_AND:
    case OP_TST:
        logical(a & b);
        break;
    case OP_EOR:
    case OP_TEQ:
        logical(a ^ b);
        break;
    case OP_ORR:
        logical(a | b);
        break;
    case OP_MOV:
        logical(b);
        break;
    case OP_BIC:
        logical(a & ~b);
        break;
    case OP_MVN:
        logical(~b);
        break;
    case OP_ADD:
    case OP_CMN: {
        auto r = detail::add32(a, b, false);
        result = r.result;
        carry = r.carry;
        overflow = r.overflow;
        break;
    }
    case OP_ADC: {
        auto r = detail::add32(a, b, carry_in);
        result = r.result;
        carry = r.carry;
        overflow = r.overflow;
        break;
    }
    case OP_SUB:
    case OP_CMP: {
        auto r = detail::add32(a, ~b, true);
        result = r.result;
        carry = r.carry;
        overflow = r.overflow;
        break;
    }
    case OP_SBC: {
        auto r = detail::add32(a, ~b, carry_in);
        result = r.result;
        carry = r.carry;
        overflow = r.overflow;
        break;
    }
    case OP_RSB: {
        auto r = detail::add32(b, ~a, true);
        result = r.result;
        carry = r.carry;
        overflow = r.overflow;
        break;
    }
    case OP_RSC: {
        auto r = detail::add32(b, ~a, carry_in);
        result = r.result;
        carry = r.carry;
        overflow = r.overflow;
        break;
    }
    default:
        break;
    }
    return {result,
            (result >> 31) != 0,
            result == 0,
            carry,
            overflow,
            !is_test_op(opcode)};
}

// ── Decoder ──────────────────────────────────────────────────────────────────
enum class InstType {
    DataProcessing,
    LoadStore,
    BlockTransfer,
    Branch,
    SWI,
    Coprocessor,
    Undefined
};

struct DecodedInstruction {
    std::uint32_t raw = 0;
    InstType inst_type = InstType::Undefined;
    std::uint32_t cond = 0;
    std::uint32_t opcode = 0;
    bool s = false;
    std::size_t rn = 0, rd = 0;
    bool immediate = false;
    std::uint32_t imm8 = 0, rotate = 0;
    std::size_t rm = 0;
    std::uint32_t shift_type = 0;
    bool shift_by_reg = false;
    std::uint32_t shift_imm = 0;
    std::size_t rs = 0;
    bool load = false, byte = false, pre_index = false, up = false,
         write_back = false;
    std::uint32_t offset12 = 0;
    std::uint16_t register_list = 0;
    bool force_user = false;
    bool link = false;
    std::int32_t branch_offset = 0;
    std::uint32_t swi_comment = 0;

    std::string disassemble() const;
};

inline DecodedInstruction decode(std::uint32_t inst) {
    DecodedInstruction d;
    d.raw = inst;
    d.cond = (inst >> 28) & 0xF;
    std::uint32_t bits2726 = (inst >> 26) & 0x3;
    std::uint32_t bit25 = (inst >> 25) & 0x1;

    if (bits2726 == 0) {
        d.inst_type = InstType::DataProcessing;
        d.immediate = ((inst >> 25) & 1) == 1;
        d.opcode = (inst >> 21) & 0xF;
        d.s = ((inst >> 20) & 1) == 1;
        d.rn = (inst >> 16) & 0xF;
        d.rd = (inst >> 12) & 0xF;
        if (d.immediate) {
            d.imm8 = inst & 0xFF;
            d.rotate = (inst >> 8) & 0xF;
        } else {
            d.rm = inst & 0xF;
            d.shift_type = (inst >> 5) & 0x3;
            d.shift_by_reg = ((inst >> 4) & 1) == 1;
            if (d.shift_by_reg) {
                d.rs = (inst >> 8) & 0xF;
            } else {
                d.shift_imm = (inst >> 7) & 0x1F;
            }
        }
    } else if (bits2726 == 1) {
        d.inst_type = InstType::LoadStore;
        d.immediate = ((inst >> 25) & 1) == 1;
        d.pre_index = ((inst >> 24) & 1) == 1;
        d.up = ((inst >> 23) & 1) == 1;
        d.byte = ((inst >> 22) & 1) == 1;
        d.write_back = ((inst >> 21) & 1) == 1;
        d.load = ((inst >> 20) & 1) == 1;
        d.rn = (inst >> 16) & 0xF;
        d.rd = (inst >> 12) & 0xF;
        if (d.immediate) {
            d.rm = inst & 0xF;
            d.shift_type = (inst >> 5) & 0x3;
            d.shift_imm = (inst >> 7) & 0x1F;
        } else {
            d.offset12 = inst & 0xFFF;
        }
    } else if (bits2726 == 2 && bit25 == 0) {
        d.inst_type = InstType::BlockTransfer;
        d.pre_index = ((inst >> 24) & 1) == 1;
        d.up = ((inst >> 23) & 1) == 1;
        d.force_user = ((inst >> 22) & 1) == 1;
        d.write_back = ((inst >> 21) & 1) == 1;
        d.load = ((inst >> 20) & 1) == 1;
        d.rn = (inst >> 16) & 0xF;
        d.register_list = static_cast<std::uint16_t>(inst & 0xFFFF);
    } else if (bits2726 == 2 && bit25 == 1) {
        d.inst_type = InstType::Branch;
        d.link = ((inst >> 24) & 1) == 1;
        std::uint32_t offset = inst & 0x00FFFFFF;
        if ((offset >> 23) != 0) offset |= 0xFF000000u;
        d.branch_offset = static_cast<std::int32_t>(offset << 2);
    } else if (bits2726 == 3) {
        if (((inst >> 24) & 0xF) == 0xF) {
            d.inst_type = InstType::SWI;
            d.swi_comment = inst & 0x00FFFFFF;
        } else {
            d.inst_type = InstType::Coprocessor;
        }
    }
    return d;
}

namespace detail {
inline std::string reg(std::size_t r) { return "R" + std::to_string(r); }
inline std::string reg_list(std::uint16_t list) {
    std::string out;
    bool first = true;
    for (int i = 0; i < 16; ++i) {
        if (((list >> i) & 1) == 0) continue;
        if (!first) out += ", ";
        first = false;
        if (i == 15)
            out += "PC";
        else if (i == 14)
            out += "LR";
        else if (i == 13)
            out += "SP";
        else
            out += "R" + std::to_string(i);
    }
    return out;
}
inline std::string operand2(const DecodedInstruction& d) {
    if (d.immediate) {
        return "#" + std::to_string(decode_immediate(d.imm8, d.rotate).value);
    }
    if (!d.shift_by_reg && d.shift_imm == 0 && d.shift_type == SHIFT_LSL) {
        return reg(d.rm);
    }
    if (d.shift_by_reg) {
        return reg(d.rm) + ", " + shift_string(d.shift_type) + " " + reg(d.rs);
    }
    std::uint32_t amount = d.shift_imm;
    if (amount == 0) {
        if (d.shift_type == SHIFT_LSR || d.shift_type == SHIFT_ASR) {
            amount = 32;
        } else if (d.shift_type == SHIFT_ROR) {
            return reg(d.rm) + ", RRX";
        }
    }
    return reg(d.rm) + ", " + shift_string(d.shift_type) + " #" +
           std::to_string(amount);
}
inline std::string hex(std::uint32_t v) {
    char buf[12];
    std::snprintf(buf, sizeof buf, "%lX", static_cast<unsigned long>(v));
    return buf;
}
inline std::string hex08(std::uint32_t v) {
    char buf[12];
    std::snprintf(buf, sizeof buf, "%08lX", static_cast<unsigned long>(v));
    return buf;
}
}  // namespace detail

inline std::string DecodedInstruction::disassemble() const {
    std::string cond = cond_string(this->cond);
    switch (inst_type) {
    case InstType::DataProcessing: {
        std::string op = op_string(opcode);
        std::string suf = (s && !is_test_op(opcode)) ? "S" : "";
        std::string op2 = detail::operand2(*this);
        if (opcode == OP_MOV || opcode == OP_MVN) {
            return op + cond + suf + " " + detail::reg(rd) + ", " + op2;
        }
        if (is_test_op(opcode)) {
            return op + cond + " " + detail::reg(rn) + ", " + op2;
        }
        return op + cond + suf + " " + detail::reg(rd) + ", " + detail::reg(rn) +
               ", " + op2;
    }
    case InstType::LoadStore: {
        std::string op = load ? "LDR" : "STR";
        std::string b_suf = byte ? "B" : "";
        std::string offset;
        if (immediate) {
            offset = detail::reg(rm);
            if (shift_imm != 0) {
                offset += ", " + shift_string(shift_type) + " #" +
                          std::to_string(shift_imm);
            }
        } else {
            offset = "#" + std::to_string(offset12);
        }
        std::string sign = up ? "" : "-";
        if (pre_index) {
            std::string wb = write_back ? "!" : "";
            return op + cond + b_suf + " " + detail::reg(rd) + ", [" +
                   detail::reg(rn) + ", " + sign + offset + "]" + wb;
        }
        return op + cond + b_suf + " " + detail::reg(rd) + ", [" +
               detail::reg(rn) + "], " + sign + offset;
    }
    case InstType::BlockTransfer: {
        std::string op = load ? "LDM" : "STM";
        std::string mode = pre_index ? (up ? "IB" : "DB") : (up ? "IA" : "DA");
        std::string wb = write_back ? "!" : "";
        return op + cond + mode + " " + detail::reg(rn) + wb + ", {" +
               detail::reg_list(register_list) + "}";
    }
    case InstType::Branch:
        return std::string(link ? "BL" : "B") + cond + " #" +
               std::to_string(branch_offset);
    case InstType::SWI:
        if (swi_comment == HALT_SWI) {
            return "HLT" + cond;
        }
        return "SWI" + cond + " #0x" + detail::hex(swi_comment);
    case InstType::Coprocessor:
        return "CDP" + cond + " (undefined)";
    case InstType::Undefined:
    default:
        return "UND" + cond + " #0x" + detail::hex08(raw);
    }
}

// ── Trace ────────────────────────────────────────────────────────────────────
struct MemoryAccess {
    std::uint32_t address;
    std::uint32_t value;
    bool operator==(const MemoryAccess& o) const {
        return address == o.address && value == o.value;
    }
};

struct Trace {
    std::uint32_t address = 0;
    std::uint32_t raw = 0;
    std::string mnemonic;
    std::string condition;
    bool condition_met = false;
    std::array<std::uint32_t, 16> regs_before{};
    std::array<std::uint32_t, 16> regs_after{};
    Flags flags_before;
    Flags flags_after;
    std::vector<MemoryAccess> memory_reads;
    std::vector<MemoryAccess> memory_writes;
};

// ── CPU ──────────────────────────────────────────────────────────────────────
class ARM1 {
  public:
    explicit ARM1(std::size_t memory_size) {
        if (memory_size == 0) memory_size = 1024 * 1024;
        memory_.assign(memory_size, 0);
        reset();
    }

    void reset() {
        regs_.fill(0);
        regs_[15] = FLAG_I | FLAG_F | MODE_SVC;
        halted_ = false;
    }

    std::uint32_t read_register(std::size_t index) const {
        if (index > 15) return 0;  // only R0-R15 are addressable
        return regs_[physical_reg(index)];
    }
    void write_register(std::size_t index, std::uint32_t value) {
        if (index > 15) return;
        regs_[physical_reg(index)] = value;
    }
    std::uint32_t pc() const { return regs_[15] & PC_MASK; }
    void set_pc(std::uint32_t addr) {
        regs_[15] = (regs_[15] & ~PC_MASK) | (addr & PC_MASK);
    }
    Flags flags() const {
        std::uint32_t r15 = regs_[15];
        return {(r15 & FLAG_N) != 0, (r15 & FLAG_Z) != 0, (r15 & FLAG_C) != 0,
                (r15 & FLAG_V) != 0};
    }
    void set_flags(Flags f) {
        std::uint32_t r15 = regs_[15] & ~(FLAG_N | FLAG_Z | FLAG_C | FLAG_V);
        if (f.n) r15 |= FLAG_N;
        if (f.z) r15 |= FLAG_Z;
        if (f.c) r15 |= FLAG_C;
        if (f.v) r15 |= FLAG_V;
        regs_[15] = r15;
    }
    std::uint32_t mode() const { return regs_[15] & MODE_MASK; }
    bool halted() const { return halted_; }
    std::uint32_t r15_raw() const { return regs_[15]; }
    const std::vector<std::uint8_t>& memory() const { return memory_; }

    std::uint32_t read_word(std::uint32_t addr) const {
        std::size_t a = static_cast<std::size_t>(addr & PC_MASK) & ~std::size_t(3);
        if (a + 3 >= memory_.size()) return 0;
        return std::uint32_t(memory_[a]) |
               (std::uint32_t(memory_[a + 1]) << 8) |
               (std::uint32_t(memory_[a + 2]) << 16) |
               (std::uint32_t(memory_[a + 3]) << 24);
    }
    void write_word(std::uint32_t addr, std::uint32_t value) {
        std::size_t a = static_cast<std::size_t>(addr & PC_MASK) & ~std::size_t(3);
        if (a + 3 >= memory_.size()) return;
        memory_[a] = static_cast<std::uint8_t>(value);
        memory_[a + 1] = static_cast<std::uint8_t>(value >> 8);
        memory_[a + 2] = static_cast<std::uint8_t>(value >> 16);
        memory_[a + 3] = static_cast<std::uint8_t>(value >> 24);
    }
    std::uint8_t read_byte(std::uint32_t addr) const {
        std::size_t a = static_cast<std::size_t>(addr & PC_MASK);
        return a < memory_.size() ? memory_[a] : 0;
    }
    void write_byte(std::uint32_t addr, std::uint8_t value) {
        std::size_t a = static_cast<std::size_t>(addr & PC_MASK);
        if (a < memory_.size()) memory_[a] = value;
    }
    void load_program(const std::vector<std::uint8_t>& code,
                      std::uint32_t start_addr) {
        for (std::size_t i = 0; i < code.size(); ++i) {
            std::size_t addr = static_cast<std::size_t>(start_addr) + i;
            if (addr < memory_.size()) memory_[addr] = code[i];
        }
    }
    void load_program_words(const std::vector<std::uint32_t>& insts,
                            std::uint32_t start_addr) {
        std::vector<std::uint8_t> code;
        code.reserve(insts.size() * 4);
        for (std::uint32_t inst : insts) {
            code.push_back(static_cast<std::uint8_t>(inst));
            code.push_back(static_cast<std::uint8_t>(inst >> 8));
            code.push_back(static_cast<std::uint8_t>(inst >> 16));
            code.push_back(static_cast<std::uint8_t>(inst >> 24));
        }
        load_program(code, start_addr);
    }

    Trace step() {
        std::uint32_t pc_val = pc();
        Trace t;
        for (std::size_t i = 0; i < 16; ++i) {
            t.regs_before[i] = read_register(i);
        }
        Flags flags_before = flags();
        t.flags_before = flags_before;
        t.address = pc_val;

        std::uint32_t instruction = read_word(pc_val);
        t.raw = instruction;
        DecodedInstruction d = decode(instruction);
        t.mnemonic = d.disassemble();
        t.condition = cond_string(d.cond);
        t.condition_met = evaluate_condition(d.cond, flags_before);

        set_pc(pc_val + 4);

        if (t.condition_met) {
            switch (d.inst_type) {
            case InstType::DataProcessing:
                exec_dp(d);
                break;
            case InstType::LoadStore:
                exec_ls(d, t);
                break;
            case InstType::BlockTransfer:
                exec_bt(d, t);
                break;
            case InstType::Branch:
                exec_branch(d);
                break;
            case InstType::SWI:
                exec_swi(d);
                break;
            default:
                trap_undefined();
                break;
            }
        }

        for (std::size_t i = 0; i < 16; ++i) {
            t.regs_after[i] = read_register(i);
        }
        t.flags_after = flags();
        return t;
    }

    std::vector<Trace> run(std::size_t max_steps) {
        std::vector<Trace> traces;
        for (std::size_t i = 0; i < max_steps; ++i) {
            if (halted_) break;
            traces.push_back(step());
        }
        return traces;
    }

  private:
    std::size_t physical_reg(std::size_t index) const {
        std::uint32_t m = mode();
        if (m == MODE_FIQ && index >= 8 && index <= 14) return 16 + (index - 8);
        if (m == MODE_IRQ && index >= 13 && index <= 14)
            return 23 + (index - 13);
        if (m == MODE_SVC && index >= 13 && index <= 14)
            return 25 + (index - 13);
        return index;
    }
    std::uint32_t read_reg_for_exec(std::size_t index) const {
        return index == 15 ? regs_[15] + 4 : read_register(index);
    }

    void exec_dp(const DecodedInstruction& d) {
        std::uint32_t a =
            (d.opcode != OP_MOV && d.opcode != OP_MVN) ? read_reg_for_exec(d.rn)
                                                       : 0;
        Flags fl = flags();
        std::uint32_t b;
        bool shifter_carry;
        if (d.immediate) {
            ShiftResult imm = decode_immediate(d.imm8, d.rotate);
            b = imm.value;
            shifter_carry = (d.rotate == 0) ? fl.c : imm.carry;
        } else {
            std::uint32_t rm_val = read_reg_for_exec(d.rm);
            std::uint32_t shift_amount =
                d.shift_by_reg ? (read_reg_for_exec(d.rs) & 0xFF) : d.shift_imm;
            ShiftResult sr =
                barrel_shift(rm_val, d.shift_type, shift_amount, fl.c,
                             d.shift_by_reg);
            b = sr.value;
            shifter_carry = sr.carry;
        }
        ALUResult r = alu_execute(d.opcode, a, b, fl.c, shifter_carry, fl.v);
        if (r.write_result) {
            if (d.rd == 15) {
                if (d.s) {
                    regs_[15] = r.result;
                } else {
                    set_pc(r.result & PC_MASK);
                }
            } else {
                write_register(d.rd, r.result);
            }
        }
        if (d.s && d.rd != 15) {
            set_flags({r.n, r.z, r.c, r.v});
        }
        if (is_test_op(d.opcode)) {
            set_flags({r.n, r.z, r.c, r.v});
        }
    }

    void exec_ls(const DecodedInstruction& d, Trace& t) {
        std::uint32_t offset;
        if (d.immediate) {
            std::uint32_t rm_val = read_reg_for_exec(d.rm);
            if (d.shift_imm != 0) {
                rm_val = barrel_shift(rm_val, d.shift_type, d.shift_imm,
                                      flags().c, false)
                             .value;
            }
            offset = rm_val;
        } else {
            offset = d.offset12;
        }
        std::uint32_t base = read_reg_for_exec(d.rn);
        std::uint32_t addr = d.up ? (base + offset) : (base - offset);
        std::uint32_t transfer_addr = d.pre_index ? addr : base;

        if (d.load) {
            std::uint32_t value;
            if (d.byte) {
                value = read_byte(transfer_addr);
            } else {
                std::uint32_t v = read_word(transfer_addr);
                std::uint32_t rotation = (transfer_addr & 3) * 8;
                if (rotation != 0) v = detail::rotr32(v, rotation);
                value = v;
            }
            t.memory_reads.push_back({transfer_addr, value});
            if (d.rd == 15) {
                regs_[15] = value;
            } else {
                write_register(d.rd, value);
            }
        } else {
            std::uint32_t value = read_reg_for_exec(d.rd);
            if (d.byte) {
                write_byte(transfer_addr, static_cast<std::uint8_t>(value & 0xFF));
            } else {
                write_word(transfer_addr, value);
            }
            t.memory_writes.push_back({transfer_addr, value});
        }

        if ((d.write_back || !d.pre_index) && d.rn != 15) {
            write_register(d.rn, addr);
        }
    }

    void exec_bt(const DecodedInstruction& d, Trace& t) {
        std::uint32_t base = read_register(d.rn);
        std::uint16_t reg_list = d.register_list;
        std::uint32_t count = 0;
        for (int i = 0; i < 16; ++i)
            if ((reg_list >> i) & 1) ++count;
        if (count == 0) return;

        std::uint32_t start_addr;
        if (!d.pre_index && d.up)
            start_addr = base;
        else if (d.pre_index && d.up)
            start_addr = base + 4;
        else if (!d.pre_index && !d.up)
            start_addr = base - (count * 4) + 4;
        else
            start_addr = base - (count * 4);

        std::uint32_t addr = start_addr;
        for (int i = 0; i < 16; ++i) {
            if (((reg_list >> i) & 1) == 0) continue;
            if (d.load) {
                std::uint32_t value = read_word(addr);
                t.memory_reads.push_back({addr, value});
                if (i == 15) {
                    regs_[15] = value;
                } else {
                    write_register(static_cast<std::size_t>(i), value);
                }
            } else {
                std::uint32_t value =
                    (i == 15) ? (regs_[15] + 4)
                              : read_register(static_cast<std::size_t>(i));
                write_word(addr, value);
                t.memory_writes.push_back({addr, value});
            }
            addr += 4;
        }
        if (d.write_back) {
            write_register(d.rn,
                           d.up ? (base + count * 4) : (base - count * 4));
        }
    }

    void exec_branch(const DecodedInstruction& d) {
        std::uint32_t branch_base = pc() + 4;
        if (d.link) write_register(14, regs_[15]);
        std::uint32_t target = static_cast<std::uint32_t>(
            static_cast<std::int32_t>(branch_base) + d.branch_offset);
        set_pc(target & PC_MASK);
    }

    void exec_swi(const DecodedInstruction& d) {
        if (d.swi_comment == HALT_SWI) {
            halted_ = true;
            return;
        }
        regs_[25] = regs_[15];
        regs_[26] = regs_[15];
        std::uint32_t r15 = regs_[15];
        r15 = (r15 & ~MODE_MASK) | MODE_SVC;
        r15 |= FLAG_I;
        regs_[15] = r15;
        set_pc(0x08);
    }

    void trap_undefined() {
        regs_[26] = regs_[15];
        std::uint32_t r15 = regs_[15];
        r15 = (r15 & ~MODE_MASK) | MODE_SVC;
        r15 |= FLAG_I;
        regs_[15] = r15;
        set_pc(0x04);
    }

    std::array<std::uint32_t, 27> regs_{};
    std::vector<std::uint8_t> memory_;
    bool halted_ = false;
};

// ── Encoding helpers ─────────────────────────────────────────────────────────
inline std::uint32_t encode_data_processing(std::uint32_t cond,
                                            std::uint32_t opcode,
                                            std::uint32_t s, std::uint32_t rn,
                                            std::uint32_t rd,
                                            std::uint32_t operand2) {
    return (cond << 28) | operand2 | (opcode << 21) | (s << 20) | (rn << 16) |
           (rd << 12);
}
inline std::uint32_t encode_mov_imm(std::uint32_t cond, std::uint32_t rd,
                                    std::uint32_t imm8) {
    return encode_data_processing(cond, OP_MOV, 0, 0, rd, (1u << 25) | imm8);
}
inline std::uint32_t encode_alu_reg(std::uint32_t cond, std::uint32_t opcode,
                                    std::uint32_t s, std::uint32_t rd,
                                    std::uint32_t rn, std::uint32_t rm) {
    return encode_data_processing(cond, opcode, s, rn, rd, rm);
}
inline std::uint32_t encode_branch(std::uint32_t cond, bool link,
                                   std::int32_t offset) {
    std::uint32_t inst = (cond << 28) | 0x0A000000u;
    if (link) inst |= 0x01000000u;
    inst |= static_cast<std::uint32_t>(offset >> 2) & 0x00FFFFFFu;
    return inst;
}
inline std::uint32_t encode_halt() {
    return (COND_AL << 28) | 0x0F000000u | HALT_SWI;
}
namespace detail {
inline std::uint32_t encode_ls(std::uint32_t base_opc, std::uint32_t cond,
                               std::uint32_t rd, std::uint32_t rn,
                               std::int32_t offset, bool pre_index) {
    std::uint32_t inst = (cond << 28) | base_opc;
    inst |= rd << 12;
    inst |= rn << 16;
    if (pre_index) inst |= 1u << 24;
    if (offset >= 0) {
        inst |= 1u << 23;
        inst |= static_cast<std::uint32_t>(offset) & 0xFFF;
    } else {
        inst |= static_cast<std::uint32_t>(-offset) & 0xFFF;
    }
    return inst;
}
}  // namespace detail
inline std::uint32_t encode_ldr(std::uint32_t cond, std::uint32_t rd,
                                std::uint32_t rn, std::int32_t offset,
                                bool pre_index) {
    return detail::encode_ls(0x04100000u, cond, rd, rn, offset, pre_index);
}
inline std::uint32_t encode_str(std::uint32_t cond, std::uint32_t rd,
                                std::uint32_t rn, std::int32_t offset,
                                bool pre_index) {
    return detail::encode_ls(0x04000000u, cond, rd, rn, offset, pre_index);
}
inline std::uint32_t encode_ldm(std::uint32_t cond, std::uint32_t rn,
                                std::uint16_t reg_list, bool write_back,
                                const std::string& mode) {
    std::uint32_t inst = (cond << 28) | 0x08100000u;
    inst |= rn << 16;
    inst |= reg_list;
    if (write_back) inst |= 1u << 21;
    if (mode == "IA")
        inst |= 1u << 23;
    else if (mode == "IB")
        inst |= (1u << 24) | (1u << 23);
    else if (mode == "DB")
        inst |= 1u << 24;
    return inst;
}
inline std::uint32_t encode_stm(std::uint32_t cond, std::uint32_t rn,
                                std::uint16_t reg_list, bool write_back,
                                const std::string& mode) {
    return encode_ldm(cond, rn, reg_list, write_back, mode) & ~(1u << 20);
}

}  // namespace arm1_simulator
}  // namespace ca

#endif  // ARM1_SIMULATOR_HPP
