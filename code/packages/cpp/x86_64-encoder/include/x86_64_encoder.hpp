// x86_64_encoder.hpp — x86-64 (AMD64) instruction encoder, header-only C++17.
// ===========================================================================
//
// A faithful port of the Rust `x86_64-encoder` crate, in namespace
// `ca::x86_64_encoder`: a stream-style assembler that produces little-endian
// x86-64 machine-code byte streams in 64-bit (long) mode — the bottom of a
// CIR → native-code lowering.
//
// Each method emits one logical instruction (1–15 bytes). Branches reference a
// `LabelId` bound to a later byte; the rel32 displacement is patched at
// `finish()` time. Cross-function / runtime references are recorded as
// `ExternalReloc`s for a later packager.
//
// V1 "always-long-form" policy: branches use `rel32`, memory operands use
// `disp32` — so the byte length of every instruction is known the moment its
// first byte is written.
//
// Encodings follow the Intel SDM Vol. 2 / AMD64 APM Vol. 3: REX prefix +
// opcode(s) + ModR/M (+ SIB) + displacement/immediate. Where the Rust crate
// returns `Result`, this port throws `ca::x86_64_encoder::Error`. Pure ISO
// C++17.

#ifndef X86_64_ENCODER_HPP
#define X86_64_ENCODER_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {
namespace x86_64_encoder {

// ── Registers (enumerator value == 4-bit register code) ──────────────────────
enum class Reg : std::uint8_t {
    Rax = 0, Rcx = 1, Rdx = 2, Rbx = 3,
    Rsp = 4, Rbp = 5, Rsi = 6, Rdi = 7,
    R8 = 8, R9 = 9, R10 = 10, R11 = 11,
    R12 = 12, R13 = 13, R14 = 14, R15 = 15
};
inline std::uint8_t code(Reg r) { return static_cast<std::uint8_t>(r); }
inline std::uint8_t low3(Reg r) { return static_cast<std::uint8_t>(r) & 0x7; }
inline bool high1(Reg r) { return static_cast<std::uint8_t>(r) >= 8; }

// ── Condition codes (4-bit tttn) ─────────────────────────────────────────────
enum class Cond : std::uint8_t {
    O = 0x0, No = 0x1, B = 0x2, Ae = 0x3, E = 0x4, Ne = 0x5, Be = 0x6, A = 0x7,
    S = 0x8, Ns = 0x9, P = 0xA, Np = 0xB, L = 0xC, Ge = 0xD, Le = 0xE, G = 0xF
};

// ── Relocations & labels ─────────────────────────────────────────────────────
enum class ExternalRelocKind { PltRel32, PcRel32, GotPcRel32 };
struct ExternalReloc {
    std::size_t patch_offset;
    std::string symbol;
    ExternalRelocKind kind;
    std::int32_t addend;
};
struct LabelId {
    std::uint32_t id;
    bool operator==(const LabelId& o) const { return id == o.id; }
};

// ── Errors ───────────────────────────────────────────────────────────────────
enum class ErrorKind { UnboundLabel, LabelAlreadyBound, BranchOutOfRange };
class Error : public std::runtime_error {
  public:
    Error(ErrorKind kind, const std::string& what)
        : std::runtime_error(what), kind_(kind) {}
    ErrorKind kind() const noexcept { return kind_; }

  private:
    ErrorKind kind_;
};

// ── Assembler ────────────────────────────────────────────────────────────────
class Assembler {
  public:
    Assembler() = default;

    std::size_t len() const { return code_.size(); }
    bool is_empty() const { return code_.empty(); }
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
            return;
        }
        auto& slot = labels_[label.id];
        if (slot.has_value()) {
            throw Error(ErrorKind::LabelAlreadyBound, "label bound twice");
        }
        slot = code_.size();
    }

    // ── MOV family ──────────────────────────────────────────────────────────
    void mov_r64_r64(Reg dst, Reg src) { emit_rr(0x89, src, dst); }
    void mov_r64_imm32(Reg dst, std::int32_t imm) {
        emit_u8(rex(true, false, false, high1(dst)));
        emit_u8(0xC7);
        emit_u8(modrm(0b11, 0, low3(dst)));
        emit_u32(static_cast<std::uint32_t>(imm));
    }
    void mov_r64_imm64(Reg dst, std::uint64_t imm) {
        emit_u8(rex(true, false, false, high1(dst)));
        emit_u8(static_cast<std::uint8_t>(0xB8 + low3(dst)));
        emit_u64(imm);
    }
    void mov_r64_mem(Reg dst, Reg base, std::int32_t disp) {
        emit_load_store(0x8B, dst, base, disp);
    }
    void mov_mem_r64(Reg base, std::int32_t disp, Reg src) {
        emit_load_store(0x89, src, base, disp);
    }
    void lea_rip_rel(Reg dst, const std::string& symbol,
                     ExternalRelocKind kind) {
        emit_u8(rex(true, high1(dst), false, false));
        emit_u8(0x8D);
        emit_u8(modrm(0b00, low3(dst), 0b101));
        std::size_t patch = code_.size();
        emit_u32(0);
        external_relocs_.push_back({patch, symbol, kind, -4});
    }

    // ── Arithmetic ──────────────────────────────────────────────────────────
    void add(Reg dst, Reg src) { emit_rr(0x01, src, dst); }
    void sub(Reg dst, Reg src) { emit_rr(0x29, src, dst); }
    void imul(Reg dst, Reg src) {
        emit_u8(rex(true, high1(dst), false, high1(src)));
        emit_u8(0x0F);
        emit_u8(0xAF);
        emit_u8(modrm(0b11, low3(dst), low3(src)));
    }
    void idiv(Reg divisor) { emit_unary_f7(divisor, 7); }
    void div(Reg divisor) { emit_unary_f7(divisor, 6); }
    void cqo() {
        emit_u8(0x48);
        emit_u8(0x99);
    }
    void add_imm32(Reg dst, std::int32_t imm) { emit_ri32(0x81, 0, dst, imm); }
    void sub_imm32(Reg dst, std::int32_t imm) { emit_ri32(0x81, 5, dst, imm); }
    void neg_(Reg dst) { emit_unary_f7(dst, 3); }

    // ── Logical ─────────────────────────────────────────────────────────────
    void and_(Reg dst, Reg src) { emit_rr(0x21, src, dst); }
    void or_(Reg dst, Reg src) { emit_rr(0x09, src, dst); }
    void xor_(Reg dst, Reg src) { emit_rr(0x31, src, dst); }
    void test_(Reg lhs, Reg rhs) { emit_rr(0x85, rhs, lhs); }
    void not_(Reg dst) { emit_unary_f7(dst, 2); }

    // ── Shifts ──────────────────────────────────────────────────────────────
    void shl_cl(Reg dst) { emit_shift_cl(dst, 4); }
    void shr_cl(Reg dst) { emit_shift_cl(dst, 5); }
    void sar_cl(Reg dst) { emit_shift_cl(dst, 7); }
    void shl_imm8(Reg dst, std::uint8_t imm) { emit_shift_imm(dst, 4, imm); }
    void shr_imm8(Reg dst, std::uint8_t imm) { emit_shift_imm(dst, 5, imm); }
    void sar_imm8(Reg dst, std::uint8_t imm) { emit_shift_imm(dst, 7, imm); }

    // ── Compare + set ───────────────────────────────────────────────────────
    void cmp(Reg lhs, Reg rhs) { emit_rr(0x39, rhs, lhs); }
    void cmp_imm32(Reg lhs, std::int32_t imm) { emit_ri32(0x81, 7, lhs, imm); }
    void setcc(Cond cond, Reg dst) {
        emit_u8(rex(false, false, false, high1(dst)));
        emit_u8(0x0F);
        emit_u8(static_cast<std::uint8_t>(0x90 |
                                          static_cast<std::uint8_t>(cond)));
        emit_u8(modrm(0b11, 0, low3(dst)));
    }
    void movzx_r64_r8(Reg dst, Reg src) {
        emit_u8(rex(true, high1(dst), false, high1(src)));
        emit_u8(0x0F);
        emit_u8(0xB6);
        emit_u8(modrm(0b11, low3(dst), low3(src)));
    }
    // Precondition: base must not be RSP/R12 (low3==4) or RBP/R13 (low3==5).
    void movzx_r64_byte_at(Reg dst, Reg base) {
        emit_u8(rex(true, high1(dst), false, high1(base)));
        emit_u8(0x0F);
        emit_u8(0xB6);
        emit_u8(modrm(0b00, low3(dst), low3(base)));
    }
    // Precondition as above.
    void mov_byte_at_r8(Reg base, Reg src) {
        emit_u8(rex(false, high1(src), false, high1(base)));
        emit_u8(0x88);
        emit_u8(modrm(0b00, low3(src), low3(base)));
    }

    // ── SSE2 scalar double ──────────────────────────────────────────────────
    void movsd_load(Reg dst_xmm, Reg base, std::int32_t disp) {
        emit_sse_mem(0xF2, 0x10, dst_xmm, base, disp);
    }
    void movsd_store(Reg base, std::int32_t disp, Reg src_xmm) {
        emit_sse_mem(0xF2, 0x11, src_xmm, base, disp);
    }
    void addsd(Reg dst, Reg src) { emit_sse_rr(0xF2, 0x58, dst, src); }
    void subsd(Reg dst, Reg src) { emit_sse_rr(0xF2, 0x5C, dst, src); }
    void mulsd(Reg dst, Reg src) { emit_sse_rr(0xF2, 0x59, dst, src); }
    void divsd(Reg dst, Reg src) { emit_sse_rr(0xF2, 0x5E, dst, src); }
    void ucomisd(Reg a, Reg b) { emit_sse_rr(0x66, 0x2E, a, b); }
    void cvtsi2sd(Reg xmm_dst, Reg gpr_src) {
        emit_sse_rr_w(0xF2, 0x2A, xmm_dst, gpr_src);
    }
    void cvttsd2si(Reg gpr_dst, Reg xmm_src) {
        emit_sse_rr_w(0xF2, 0x2C, gpr_dst, xmm_src);
    }
    void roundsd(Reg xmm_dst, Reg xmm_src, std::uint8_t imm8) {
        emit_sse_rri_0f3a(0x0B, xmm_dst, xmm_src, imm8);
    }
    void sqrtsd(Reg xmm_dst, Reg xmm_src) {
        emit_sse_rr(0xF2, 0x51, xmm_dst, xmm_src);
    }

    // ── Stack ───────────────────────────────────────────────────────────────
    void push(Reg src) {
        if (high1(src)) {
            emit_u8(rex(false, false, false, true));
        }
        emit_u8(static_cast<std::uint8_t>(0x50 + low3(src)));
    }
    void pop(Reg dst) {
        if (high1(dst)) {
            emit_u8(rex(false, false, false, true));
        }
        emit_u8(static_cast<std::uint8_t>(0x58 + low3(dst)));
    }

    // ── Control flow ────────────────────────────────────────────────────────
    void jmp(LabelId target) {
        emit_u8(0xE9);
        emit_branch_slot(target);
    }
    void jcc(Cond cond, LabelId target) {
        emit_u8(0x0F);
        emit_u8(static_cast<std::uint8_t>(0x80 |
                                          static_cast<std::uint8_t>(cond)));
        emit_branch_slot(target);
    }
    void call_rel32(const std::string& symbol, ExternalRelocKind kind) {
        emit_u8(0xE8);
        std::size_t patch = code_.size();
        emit_u32(0);
        external_relocs_.push_back({patch, symbol, kind, -4});
    }
    void call_label(LabelId target) {
        emit_u8(0xE8);
        emit_branch_slot(target);
    }
    void call_r64(Reg target) {
        emit_u8(rex(false, false, false, high1(target)));
        emit_u8(0xFF);
        emit_u8(modrm(0b11, 2, low3(target)));
    }
    void ret() { emit_u8(0xC3); }

    // ── Misc ────────────────────────────────────────────────────────────────
    void nop() { emit_u8(0x90); }
    void int3() { emit_u8(0xCC); }
    void ud2() {
        emit_u8(0x0F);
        emit_u8(0x0B);
    }

    // ── Finalisation ────────────────────────────────────────────────────────
    std::vector<std::uint8_t> finish() {
        for (const auto& f : fixups_) {
            if (f.target.id >= labels_.size() ||
                !labels_[f.target.id].has_value()) {
                throw Error(ErrorKind::UnboundLabel,
                            "label referenced but never bound");
            }
            std::int64_t target =
                static_cast<std::int64_t>(*labels_[f.target.id]);
            std::int64_t delta =
                target - static_cast<std::int64_t>(f.instr_end_offset);
            if (delta < INT32_MIN || delta > INT32_MAX) {
                throw Error(ErrorKind::BranchOutOfRange,
                            "branch displacement doesn't fit in 32 bits");
            }
            std::uint32_t d =
                static_cast<std::uint32_t>(static_cast<std::int32_t>(delta));
            code_[f.slot_offset + 0] = static_cast<std::uint8_t>(d & 0xFF);
            code_[f.slot_offset + 1] =
                static_cast<std::uint8_t>((d >> 8) & 0xFF);
            code_[f.slot_offset + 2] =
                static_cast<std::uint8_t>((d >> 16) & 0xFF);
            code_[f.slot_offset + 3] =
                static_cast<std::uint8_t>((d >> 24) & 0xFF);
        }
        return code_;
    }

  private:
    struct Fixup {
        std::size_t slot_offset;
        std::size_t instr_end_offset;
        LabelId target;
    };

    void emit_u8(std::uint8_t b) { code_.push_back(b); }
    void emit_u32(std::uint32_t w) {
        for (int i = 0; i < 4; ++i) {
            code_.push_back(static_cast<std::uint8_t>((w >> (8 * i)) & 0xFF));
        }
    }
    void emit_u64(std::uint64_t w) {
        for (int i = 0; i < 8; ++i) {
            code_.push_back(static_cast<std::uint8_t>((w >> (8 * i)) & 0xFF));
        }
    }
    void emit_branch_slot(LabelId target) {
        std::size_t slot = code_.size();
        emit_u32(0);
        fixups_.push_back({slot, code_.size(), target});
    }

    static std::uint8_t rex(bool w, bool r, bool x, bool b) {
        return static_cast<std::uint8_t>(
            0x40 | (static_cast<std::uint8_t>(w) << 3) |
            (static_cast<std::uint8_t>(r) << 2) |
            (static_cast<std::uint8_t>(x) << 1) | static_cast<std::uint8_t>(b));
    }
    static std::uint8_t modrm(std::uint8_t mode, std::uint8_t reg,
                              std::uint8_t rm) {
        return static_cast<std::uint8_t>((mode << 6) | (reg << 3) | rm);
    }

    void emit_rr(std::uint8_t opcode, Reg reg_src, Reg rm_dst) {
        emit_u8(rex(true, high1(reg_src), false, high1(rm_dst)));
        emit_u8(opcode);
        emit_u8(modrm(0b11, low3(reg_src), low3(rm_dst)));
    }
    void emit_ri32(std::uint8_t opcode, std::uint8_t ext, Reg dst,
                   std::int32_t imm) {
        emit_u8(rex(true, false, false, high1(dst)));
        emit_u8(opcode);
        emit_u8(modrm(0b11, ext, low3(dst)));
        emit_u32(static_cast<std::uint32_t>(imm));
    }
    void emit_unary_f7(Reg dst, std::uint8_t ext) {
        emit_u8(rex(true, false, false, high1(dst)));
        emit_u8(0xF7);
        emit_u8(modrm(0b11, ext, low3(dst)));
    }
    void emit_load_store(std::uint8_t opcode, Reg reg, Reg base,
                         std::int32_t disp) {
        bool needs_sib = low3(base) == 4;
        emit_u8(rex(true, high1(reg), false, high1(base)));
        emit_u8(opcode);
        if (needs_sib) {
            emit_u8(modrm(0b10, low3(reg), 0b100));
            emit_u8(static_cast<std::uint8_t>((0b100 << 3) | low3(base)));
        } else {
            emit_u8(modrm(0b10, low3(reg), low3(base)));
        }
        emit_u32(static_cast<std::uint32_t>(disp));
    }
    void emit_shift_cl(Reg dst, std::uint8_t ext) {
        emit_u8(rex(true, false, false, high1(dst)));
        emit_u8(0xD3);
        emit_u8(modrm(0b11, ext, low3(dst)));
    }
    void emit_shift_imm(Reg dst, std::uint8_t ext, std::uint8_t imm) {
        emit_u8(rex(true, false, false, high1(dst)));
        emit_u8(0xC1);
        emit_u8(modrm(0b11, ext, low3(dst)));
        emit_u8(imm);
    }
    void emit_sse_rr(std::uint8_t prefix, std::uint8_t opcode, Reg dst,
                     Reg src) {
        emit_u8(prefix);
        if (high1(dst) || high1(src)) {
            emit_u8(rex(false, high1(dst), false, high1(src)));
        }
        emit_u8(0x0F);
        emit_u8(opcode);
        emit_u8(modrm(0b11, low3(dst), low3(src)));
    }
    void emit_sse_mem(std::uint8_t prefix, std::uint8_t opcode, Reg xmm,
                      Reg base, std::int32_t disp) {
        emit_u8(prefix);
        if (high1(xmm) || high1(base)) {
            emit_u8(rex(false, high1(xmm), false, high1(base)));
        }
        emit_u8(0x0F);
        emit_u8(opcode);
        bool needs_sib = low3(base) == 4;
        if (needs_sib) {
            emit_u8(modrm(0b10, low3(xmm), 0b100));
            emit_u8(static_cast<std::uint8_t>((0b100 << 3) | low3(base)));
        } else {
            emit_u8(modrm(0b10, low3(xmm), low3(base)));
        }
        emit_u32(static_cast<std::uint32_t>(disp));
    }
    void emit_sse_rr_w(std::uint8_t prefix, std::uint8_t opcode, Reg reg,
                       Reg rm) {
        emit_u8(prefix);
        emit_u8(rex(true, high1(reg), false, high1(rm)));
        emit_u8(0x0F);
        emit_u8(opcode);
        emit_u8(modrm(0b11, low3(reg), low3(rm)));
    }
    void emit_sse_rri_0f3a(std::uint8_t opcode, Reg reg, Reg rm,
                           std::uint8_t imm8) {
        emit_u8(0x66);
        if (high1(reg) || high1(rm)) {
            emit_u8(rex(false, high1(reg), false, high1(rm)));
        }
        emit_u8(0x0F);
        emit_u8(0x3A);
        emit_u8(opcode);
        emit_u8(modrm(0b11, low3(reg), low3(rm)));
        emit_u8(imm8);
    }

    std::vector<std::uint8_t> code_;
    std::vector<std::optional<std::size_t>> labels_;
    std::vector<Fixup> fixups_;
    std::vector<ExternalReloc> external_relocs_;
};

}  // namespace x86_64_encoder
}  // namespace ca

#endif  // X86_64_ENCODER_HPP
