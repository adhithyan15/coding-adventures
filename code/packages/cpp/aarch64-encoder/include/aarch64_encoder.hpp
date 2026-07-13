// aarch64_encoder.hpp — AArch64 (ARM64) instruction encoder, header-only C++17.
// =============================================================================
//
// A faithful port of the Rust `aarch64-encoder` crate, in namespace
// `ca::aarch64_encoder`: a stream-style assembler that produces little-endian
// 32-bit instruction words for the AArch64 instruction set (the bottom of a
// CIR → native-code lowering).
//
// Each method on `Assembler` emits one 4-byte instruction word. Branches
// reference `LabelId`s bound to a later instruction; the displacement is patched
// at `finish()` time. `finish()` returns the raw `.text` byte stream.
//
// Every encoding follows the ARM Architecture Reference Manual (DDI 0487) bit
// layout: an instruction word is a fixed base opcode OR'd with register fields
// (`Rd` in [4:0], `Rn` in [9:5], `Rm` in [20:16], …) and immediates.
//
// Where the Rust crate returns `Result`, this port throws
// `ca::aarch64_encoder::Error` carrying an `ErrorKind`. Pure ISO C++17.

#ifndef AARCH64_ENCODER_HPP
#define AARCH64_ENCODER_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace aarch64_encoder {

// ── Registers ────────────────────────────────────────────────────────────────
// The enumerator value IS the 5-bit register index. Sp and Xzr share code 31;
// the instruction opcode (not the register field) picks the interpretation.
enum class Reg : std::uint32_t {
    X0 = 0, X1, X2, X3, X4, X5, X6, X7,
    X8, X9, X10, X11, X12, X13, X14, X15,
    X16, X17, X18, X19, X20, X21, X22, X23,
    X24, X25, X26, X27, X28,
    Fp = 29,  // frame pointer (X29)
    Lr = 30,  // link register (X30)
    Sp = 31,  // stack pointer
    Xzr = 31  // zero register (same code as Sp)
};

inline std::uint32_t idx(Reg r) { return static_cast<std::uint32_t>(r); }

// ── Condition codes (4-bit) ──────────────────────────────────────────────────
enum class Cond : std::uint32_t {
    Eq = 0, Ne, Hs, Lo, Mi, Pl, Vs, Vc, Hi, Ls, Ge, Lt, Gt, Le, Al
};

// ── Labels ───────────────────────────────────────────────────────────────────
struct LabelId {
    std::uint32_t id;
    bool operator==(const LabelId& o) const { return id == o.id; }
};

// ── Errors ───────────────────────────────────────────────────────────────────
enum class ErrorKind {
    UnboundLabel,
    LabelAlreadyBound,
    ImmediateOutOfRange,
    BranchOutOfRange
};

class Error : public std::runtime_error {
  public:
    Error(ErrorKind kind, const std::string& what)
        : std::runtime_error(what), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

  private:
    ErrorKind kind_;
};

// A pending cross-function BL relocation.
struct ExternalReloc {
    std::size_t word_idx;
    std::string symbol;
};

// ── Assembler ────────────────────────────────────────────────────────────────
class Assembler {
  public:
    Assembler() = default;

    std::size_t len_words() const { return code_.size(); }
    const std::vector<ExternalReloc>& external_relocs() const {
        return external_relocs_;
    }

    // ── Labels ──────────────────────────────────────────────────────────────
    LabelId create_label() {
        LabelId id{static_cast<std::uint32_t>(labels_.size())};
        labels_.push_back(std::nullopt);
        return id;
    }
    void bind(LabelId label) {
        if (label.id >= labels_.size()) {
            return;  // foreign/fabricated id — ignore (matches the C port)
        }
        auto& slot = labels_[label.id];
        if (slot.has_value()) {
            throw Error(ErrorKind::LabelAlreadyBound, "label bound twice");
        }
        slot = code_.size();
    }

    // ── Move-immediate ──────────────────────────────────────────────────────
    void movz(Reg rd, std::uint16_t imm16, std::uint8_t hw) {
        // hw in 0..=3 selects the shift 0/16/32/48.
        emit(0xD2800000u | (static_cast<std::uint32_t>(hw & 3) << 21) |
             (static_cast<std::uint32_t>(imm16) << 5) | idx(rd));
    }
    void movk(Reg rd, std::uint16_t imm16, std::uint8_t hw) {
        emit(0xF2800000u | (static_cast<std::uint32_t>(hw & 3) << 21) |
             (static_cast<std::uint32_t>(imm16) << 5) | idx(rd));
    }
    // Load a 64-bit immediate with the shortest MOVZ/MOVK sequence.
    void mov_imm64(Reg rd, std::uint64_t imm) {
        if (imm == 0) {
            movz(rd, 0, 0);
            return;
        }
        bool emitted = false;
        for (int i = 0; i < 4; ++i) {
            std::uint16_t c =
                static_cast<std::uint16_t>((imm >> (16 * i)) & 0xFFFF);
            if (c == 0) continue;
            if (!emitted) {
                movz(rd, c, static_cast<std::uint8_t>(i));
                emitted = true;
            } else {
                movk(rd, c, static_cast<std::uint8_t>(i));
            }
        }
    }

    // ── Arithmetic (register) ───────────────────────────────────────────────
    void add(Reg rd, Reg rn, Reg rm) { emit_r(0x8B000000u, rd, rn, rm); }
    void sub(Reg rd, Reg rn, Reg rm) { emit_r(0xCB000000u, rd, rn, rm); }
    void mul(Reg rd, Reg rn, Reg rm) {
        // MADD Xd, Xn, Xm, XZR.
        emit(0x9B000000u | (idx(rm) << 16) | (0x1Fu << 10) | (idx(rn) << 5) |
             idx(rd));
    }

    // ── Division / multiply-subtract ────────────────────────────────────────
    void sdiv(Reg rd, Reg rn, Reg rm) { emit_r(0x9AC00C00u, rd, rn, rm); }
    void udiv(Reg rd, Reg rn, Reg rm) { emit_r(0x9AC00800u, rd, rn, rm); }
    void msub(Reg rd, Reg rn, Reg rm, Reg ra) {
        emit(0x9B008000u | (idx(rm) << 16) | (idx(ra) << 10) | (idx(rn) << 5) |
             idx(rd));
    }

    // ── Logical (register) ──────────────────────────────────────────────────
    void and_(Reg rd, Reg rn, Reg rm) { emit_r(0x8A000000u, rd, rn, rm); }
    void orr(Reg rd, Reg rn, Reg rm) { emit_r(0xAA000000u, rd, rn, rm); }
    void eor(Reg rd, Reg rn, Reg rm) { emit_r(0xCA000000u, rd, rn, rm); }
    void mvn(Reg rd, Reg rm) {
        // ORN Xd, XZR, Xm.
        emit(0xAA200000u | (idx(rm) << 16) | (0x1Fu << 5) | idx(rd));
    }

    // ── Variable shifts (register) ──────────────────────────────────────────
    void lsl_reg(Reg rd, Reg rn, Reg rm) { emit_r(0x9AC02000u, rd, rn, rm); }
    void lsr_reg(Reg rd, Reg rn, Reg rm) { emit_r(0x9AC02400u, rd, rn, rm); }
    void asr_reg(Reg rd, Reg rn, Reg rm) { emit_r(0x9AC02800u, rd, rn, rm); }

    // ── Unary ───────────────────────────────────────────────────────────────
    void neg_(Reg rd, Reg rm) {
        // SUB Xd, XZR, Xm.
        emit(0xCB000000u | (idx(rm) << 16) | (0x1Fu << 5) | idx(rd));
    }

    // ── PC-relative page placeholder ────────────────────────────────────────
    std::size_t adrp_placeholder(Reg rd) {
        std::size_t word_idx = code_.size();
        emit(0x90000000u | idx(rd));
        return word_idx;
    }

    // ── Arithmetic (immediate) ──────────────────────────────────────────────
    void add_imm(Reg rd, Reg rn, std::uint32_t imm12) {
        check_imm12("add_imm", imm12);
        emit(0x91000000u | (imm12 << 10) | (idx(rn) << 5) | idx(rd));
    }
    void sub_imm(Reg rd, Reg rn, std::uint32_t imm12) {
        check_imm12("sub_imm", imm12);
        emit(0xD1000000u | (imm12 << 10) | (idx(rn) << 5) | idx(rd));
    }

    // ── Compare ─────────────────────────────────────────────────────────────
    void cmp(Reg rn, Reg rm) {
        emit(0xEB000000u | (idx(rm) << 16) | (idx(rn) << 5) | 0x1Fu);
    }
    void cmp_imm(Reg rn, std::uint32_t imm12) {
        check_imm12("cmp_imm", imm12);
        emit(0xF1000000u | (imm12 << 10) | (idx(rn) << 5) | 0x1Fu);
    }

    // ── Memory (scaled unsigned offset, 64-bit) ─────────────────────────────
    void ldr(Reg rt, Reg rn, std::uint32_t imm) {
        emit(mem_scaled8("ldr", 0xF9400000u, rt, rn, imm));
    }
    void str_(Reg rt, Reg rn, std::uint32_t imm) {
        emit(mem_scaled8("str", 0xF9000000u, rt, rn, imm));
    }
    void ldr_d(Reg dt, Reg rn, std::uint32_t imm) {
        emit(mem_scaled8("ldr_d", 0xFD400000u, dt, rn, imm));
    }
    void str_d(Reg dt, Reg rn, std::uint32_t imm) {
        emit(mem_scaled8("str_d", 0xFD000000u, dt, rn, imm));
    }

    // ── Scalar double-precision FP ──────────────────────────────────────────
    void fadd(Reg dd, Reg dn, Reg dm) { emit_r(0x1E602800u, dd, dn, dm); }
    void fsub(Reg dd, Reg dn, Reg dm) { emit_r(0x1E603800u, dd, dn, dm); }
    void fmul(Reg dd, Reg dn, Reg dm) { emit_r(0x1E600800u, dd, dn, dm); }
    void fdiv(Reg dd, Reg dn, Reg dm) { emit_r(0x1E601800u, dd, dn, dm); }
    void fcmp(Reg dn, Reg dm) {
        emit(0x1E602000u | (idx(dm) << 16) | (idx(dn) << 5));
    }

    // ── int ⇄ real conversions ──────────────────────────────────────────────
    void scvtf(Reg dd, Reg xn) {
        emit(0x9E620000u | (idx(xn) << 5) | idx(dd));
    }
    void fcvtzs(Reg xd, Reg dn) {
        emit(0x9E780000u | (idx(dn) << 5) | idx(xd));
    }
    void frintm(Reg dd, Reg dn) {
        emit(0x1E654000u | (idx(dn) << 5) | idx(dd));
    }
    void fsqrt(Reg dd, Reg dn) {
        emit(0x1E61C000u | (idx(dn) << 5) | idx(dd));
    }

    // ── Byte memory ─────────────────────────────────────────────────────────
    void ldrb(Reg rt, Reg rn, std::uint32_t imm) {
        if (imm > 0xFFF) {
            throw imm_error("ldrb", 12, imm);
        }
        emit(0x39400000u | (imm << 10) | (idx(rn) << 5) | idx(rt));
    }
    void strb(Reg rt, Reg rn, std::uint32_t imm) {
        if (imm > 0xFFF) {
            throw imm_error("strb", 12, imm);
        }
        emit(0x39000000u | (imm << 10) | (idx(rn) << 5) | idx(rt));
    }
    void strb_pre_neg1(Reg wt, Reg rn) {
        emit(0x381FFC00u | (idx(rn) << 5) | idx(wt));
    }

    // ── STP / LDP (prologue/epilogue) ───────────────────────────────────────
    void stp_pre(Reg rt1, Reg rt2, Reg rn, std::int32_t imm) {
        emit(pair_imm7("stp_pre", 0xA9800000u, rt1, rt2, rn, imm));
    }
    void ldp_post(Reg rt1, Reg rt2, Reg rn, std::int32_t imm) {
        emit(pair_imm7("ldp_post", 0xA8C00000u, rt1, rt2, rn, imm));
    }

    // ── Branches ────────────────────────────────────────────────────────────
    void b(LabelId target) {
        emit_branch(target, BranchKind::Imm26, 0x14000000u);
    }
    void bl(LabelId target) {
        emit_branch(target, BranchKind::Imm26, 0x94000000u);
    }
    std::size_t bl_external(const std::string& symbol) {
        std::size_t word_idx = code_.size();
        emit(0x94000000u);
        external_relocs_.push_back({word_idx, symbol});
        return word_idx;
    }
    void b_cond(Cond cond, LabelId target) {
        emit_branch(target, BranchKind::Imm19,
                    0x54000000u | static_cast<std::uint32_t>(cond));
    }
    void blr(Reg rn) { emit(0xD63F0000u | (idx(rn) << 5)); }
    void ret() { emit(0xD65F0000u | (idx(Reg::Lr) << 5)); }

    // ── Misc ────────────────────────────────────────────────────────────────
    void cset(Reg rd, Cond cond) {
        std::uint32_t cc = static_cast<std::uint32_t>(cond);
        std::uint32_t inv = cc ^ 1u;
        emit(0x9A800400u | (0x1Fu << 16) | (inv << 12) | (0x1Fu << 5) |
             idx(rd));
    }
    void cbz(Reg rt, LabelId target) {
        emit_branch(target, BranchKind::Imm19, 0xB4000000u | idx(rt));
    }
    void cbnz(Reg rt, LabelId target) {
        emit_branch(target, BranchKind::Imm19, 0xB5000000u | idx(rt));
    }
    void nop() { emit(0xD503201Fu); }
    void udf(std::uint16_t imm) { emit(static_cast<std::uint32_t>(imm)); }
    void svc(std::uint16_t imm) {
        emit(0xD4000001u | (static_cast<std::uint32_t>(imm) << 5));
    }

    // ── Finalisation ────────────────────────────────────────────────────────
    std::vector<std::uint8_t> finish() {
        for (const auto& f : fixups_) {
            if (f.target.id >= labels_.size() ||
                !labels_[f.target.id].has_value()) {
                throw Error(ErrorKind::UnboundLabel,
                            "label referenced but never bound");
            }
            const auto& slot = labels_[f.target.id];
            std::int64_t delta = static_cast<std::int64_t>(*slot) -
                                 static_cast<std::int64_t>(f.word_idx);
            std::uint32_t& word = code_[f.word_idx];
            if (f.kind == BranchKind::Imm26) {
                if (delta < -(1 << 25) || delta >= (1 << 25)) {
                    throw branch_error(26, delta);
                }
                std::uint32_t imm26 =
                    static_cast<std::uint32_t>(delta) & 0x03FFFFFFu;
                word = (word & ~0x03FFFFFFu) | imm26;
            } else {
                if (delta < -(1 << 18) || delta >= (1 << 18)) {
                    throw branch_error(19, delta);
                }
                std::uint32_t imm19 =
                    static_cast<std::uint32_t>(delta) & 0x0007FFFFu;
                word = (word & ~(0x0007FFFFu << 5)) | (imm19 << 5);
            }
        }
        std::vector<std::uint8_t> bytes;
        bytes.reserve(code_.size() * 4);
        for (std::uint32_t w : code_) {
            bytes.push_back(static_cast<std::uint8_t>(w & 0xFF));
            bytes.push_back(static_cast<std::uint8_t>((w >> 8) & 0xFF));
            bytes.push_back(static_cast<std::uint8_t>((w >> 16) & 0xFF));
            bytes.push_back(static_cast<std::uint8_t>((w >> 24) & 0xFF));
        }
        return bytes;
    }

  private:
    enum class BranchKind { Imm26, Imm19 };
    struct Fixup {
        std::size_t word_idx;
        LabelId target;
        BranchKind kind;
    };

    void emit(std::uint32_t word) { code_.push_back(word); }
    void emit_r(std::uint32_t base, Reg rd, Reg rn, Reg rm) {
        emit(base | (idx(rm) << 16) | (idx(rn) << 5) | idx(rd));
    }
    void emit_branch(LabelId target, BranchKind kind, std::uint32_t base) {
        std::size_t word_idx = code_.size();
        emit(base);
        fixups_.push_back({word_idx, target, kind});
    }

    static Error imm_error(const char* op, std::uint32_t bits,
                           std::int64_t value) {
        return Error(ErrorKind::ImmediateOutOfRange,
                     std::string(op) + ": immediate " + std::to_string(value) +
                         " doesn't fit in " + std::to_string(bits) + " bits");
    }
    static Error branch_error(std::uint32_t bits, std::int64_t delta) {
        return Error(ErrorKind::BranchOutOfRange,
                     "branch displacement " + std::to_string(delta) +
                         " words doesn't fit in " + std::to_string(bits) +
                         " bits");
    }
    static void check_imm12(const char* op, std::uint32_t imm12) {
        if (imm12 >= (1u << 12)) {
            throw imm_error(op, 12, imm12);
        }
    }
    static std::uint32_t mem_scaled8(const char* op, std::uint32_t base, Reg rt,
                                     Reg rn, std::uint32_t imm) {
        if (imm % 8 != 0 || imm > 0x7FF8) {
            throw imm_error(op, 12, imm);
        }
        std::uint32_t imm12 = imm / 8;
        return base | (imm12 << 10) | (idx(rn) << 5) | idx(rt);
    }
    static std::uint32_t pair_imm7(const char* op, std::uint32_t base, Reg rt1,
                                   Reg rt2, Reg rn, std::int32_t imm) {
        if (imm % 8 != 0 || imm < -512 || imm > 504) {
            throw imm_error(op, 7, imm);
        }
        std::uint32_t imm7 = static_cast<std::uint32_t>(imm / 8) & 0x7Fu;
        return base | (imm7 << 15) | (idx(rt2) << 10) | (idx(rn) << 5) |
               idx(rt1);
    }

    std::vector<std::uint32_t> code_;
    std::vector<std::optional<std::size_t>> labels_;
    std::vector<Fixup> fixups_;
    std::vector<ExternalReloc> external_relocs_;
};

}  // namespace aarch64_encoder
}  // namespace ca

#endif  // AARCH64_ENCODER_HPP
