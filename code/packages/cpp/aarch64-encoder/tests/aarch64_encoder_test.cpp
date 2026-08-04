// Tests for aarch64-encoder, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests (known-good ARM64 encodings).
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "aarch64_encoder.hpp"

namespace a64 = ca::aarch64_encoder;
using Reg = a64::Reg;
using Cond = a64::Cond;

// The single word an assembler produced (asserts it is exactly one word).
static std::uint32_t one_word(a64::Assembler& a) {
    auto b = a.finish();
    if (b.size() != 4) {
        return 0xDEADBEEFu;
    }
    return static_cast<std::uint32_t>(b[0]) |
           (static_cast<std::uint32_t>(b[1]) << 8) |
           (static_cast<std::uint32_t>(b[2]) << 16) |
           (static_cast<std::uint32_t>(b[3]) << 24);
}
static std::uint32_t word_at(const std::vector<std::uint8_t>& b, std::size_t i) {
    return static_cast<std::uint32_t>(b[i]) |
           (static_cast<std::uint32_t>(b[i + 1]) << 8) |
           (static_cast<std::uint32_t>(b[i + 2]) << 16) |
           (static_cast<std::uint32_t>(b[i + 3]) << 24);
}

int main() {
    // ── moves ──────────────────────────────────────────────────────────────
    { a64::Assembler a; a.movz(Reg::X0, 0, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xD2800000u); }
    { a64::Assembler a; a.movz(Reg::X1, 5, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xD28000A1u); }
    { a64::Assembler a; a.movz(Reg::X0, 0x1234, 1); ISO_CHECK_EQ_UINT(one_word(a), 0xD2A24680u); }
    { a64::Assembler a; a.mov_imm64(Reg::X0, 0); ISO_CHECK_EQ_UINT(a.finish().size(), 4u); }
    { a64::Assembler a; a.mov_imm64(Reg::X0, 5); ISO_CHECK_EQ_UINT(one_word(a), 0xD28000A0u); }
    { a64::Assembler a; a.mov_imm64(Reg::X0, 0x12345678u); ISO_CHECK_EQ_UINT(a.finish().size(), 8u); }

    // ── arithmetic (register) ──────────────────────────────────────────────
    { a64::Assembler a; a.add(Reg::X0, Reg::X0, Reg::X1); ISO_CHECK_EQ_UINT(one_word(a), 0x8B010000u); }
    { a64::Assembler a; a.sub(Reg::X2, Reg::X3, Reg::X4); ISO_CHECK_EQ_UINT(one_word(a), 0xCB040062u); }
    { a64::Assembler a; a.mul(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9B027C20u); }

    // ── arithmetic (immediate) ─────────────────────────────────────────────
    { a64::Assembler a; a.add_imm(Reg::X0, Reg::X0, 1); ISO_CHECK_EQ_UINT(one_word(a), 0x91000400u); }
    {
        a64::Assembler a;
        bool threw = false;
        try { a.add_imm(Reg::X0, Reg::X0, 1u << 12); }
        catch (const a64::Error& e) { threw = true; ISO_CHECK(e.kind() == a64::ErrorKind::ImmediateOutOfRange); }
        ISO_CHECK(threw);
    }

    // ── compare ────────────────────────────────────────────────────────────
    { a64::Assembler a; a.cmp_imm(Reg::X0, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xF100001Fu); }
    { a64::Assembler a; a.cmp(Reg::X0, Reg::X1); ISO_CHECK_EQ_UINT(one_word(a), 0xEB01001Fu); }

    // ── memory ─────────────────────────────────────────────────────────────
    { a64::Assembler a; a.ldr(Reg::X0, Reg::Sp, 0); ISO_CHECK_EQ_UINT(one_word(a), 0xF94003E0u); }
    { a64::Assembler a; a.str_(Reg::X0, Reg::Sp, 8); ISO_CHECK_EQ_UINT(one_word(a), 0xF90007E0u); }
    {
        a64::Assembler a;
        bool threw = false;
        try { a.ldr(Reg::X0, Reg::Sp, 7); } catch (const a64::Error&) { threw = true; }
        ISO_CHECK(threw);
    }

    // ── scalar FP ──────────────────────────────────────────────────────────
    {
        a64::Assembler a;
        a.ldr_d(Reg::X0, Reg::Sp, 8);
        a.str_d(Reg::X0, Reg::Sp, 8);
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0xFD4007E0u);
        ISO_CHECK_EQ_UINT(word_at(b, 4), 0xFD0007E0u);
    }
    {
        a64::Assembler a;
        a.fadd(Reg::X0, Reg::X0, Reg::X1);
        a.fsub(Reg::X0, Reg::X0, Reg::X1);
        a.fmul(Reg::X0, Reg::X0, Reg::X1);
        a.fdiv(Reg::X0, Reg::X0, Reg::X1);
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0x1E612800u);
        ISO_CHECK_EQ_UINT(word_at(b, 4), 0x1E613800u);
        ISO_CHECK_EQ_UINT(word_at(b, 8), 0x1E610800u);
        ISO_CHECK_EQ_UINT(word_at(b, 12), 0x1E611800u);
    }
    { a64::Assembler a; a.fcmp(Reg::X0, Reg::X1); ISO_CHECK_EQ_UINT(one_word(a), 0x1E612000u); }
    { a64::Assembler a; a.fsqrt(Reg::X0, Reg::X0); ISO_CHECK_EQ_UINT(one_word(a), 0x1E61C000u); }

    // ── int ⇄ real conversions ─────────────────────────────────────────────
    {
        a64::Assembler a;
        a.scvtf(Reg::X0, Reg::X3);
        a.fcvtzs(Reg::X2, Reg::X0);
        a.frintm(Reg::X0, Reg::X1);
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0x9E620060u);
        ISO_CHECK_EQ_UINT(word_at(b, 4), 0x9E780002u);
        ISO_CHECK_EQ_UINT(word_at(b, 8), 0x1E654020u);
    }

    // ── STP / LDP ──────────────────────────────────────────────────────────
    { a64::Assembler a; a.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -16); ISO_CHECK_EQ_UINT(one_word(a), 0xA9BF7BFDu); }
    { a64::Assembler a; a.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, 16); ISO_CHECK_EQ_UINT(one_word(a), 0xA8C17BFDu); }

    // ── branches ───────────────────────────────────────────────────────────
    {
        a64::Assembler a;
        auto l2 = a.create_label();
        a.movz(Reg::X0, 1, 0);
        a.b(l2);
        a.movz(Reg::X0, 2, 0);
        a.bind(l2);
        a.ret();
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(b.size(), 16u);
        ISO_CHECK_EQ_UINT(word_at(b, 4), 0x14000002u);
    }
    {
        a64::Assembler a;
        auto lp = a.create_label();
        a.bind(lp);
        a.nop();
        a.b(lp);
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 4), 0x14000000u | 0x03FFFFFFu);
    }
    {
        a64::Assembler a;
        auto l = a.create_label();
        a.cmp_imm(Reg::X0, 0);
        a.b_cond(Cond::Eq, l);
        a.nop();
        a.bind(l);
        a.ret();
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 4),
                          0x54000000u | (2u << 5) |
                              static_cast<std::uint32_t>(Cond::Eq));
    }
    {
        a64::Assembler a;
        auto l = a.create_label();
        a.b(l);
        bool threw = false;
        try { a.finish(); } catch (const a64::Error& e) { threw = true; ISO_CHECK(e.kind() == a64::ErrorKind::UnboundLabel); }
        ISO_CHECK(threw);
    }
    {
        a64::Assembler a;
        auto l = a.create_label();
        a.bind(l);
        bool threw = false;
        try { a.bind(l); } catch (const a64::Error& e) { threw = true; ISO_CHECK(e.kind() == a64::ErrorKind::LabelAlreadyBound); }
        ISO_CHECK(threw);
    }
    {
        a64::Assembler a;
        auto l = a.create_label();
        a.bl(l);
        a.bind(l);
        a.ret();
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0x94000001u);
    }

    // ── indirect / return / misc ───────────────────────────────────────────
    { a64::Assembler a; a.blr(Reg::X0); ISO_CHECK_EQ_UINT(one_word(a), 0xD63F0000u); }
    { a64::Assembler a; a.ret(); ISO_CHECK_EQ_UINT(one_word(a), 0xD65F03C0u); }
    { a64::Assembler a; a.nop(); ISO_CHECK_EQ_UINT(one_word(a), 0xD503201Fu); }
    { a64::Assembler a; a.udf(0); ISO_CHECK_EQ_UINT(one_word(a), 0x00000000u); }
    { a64::Assembler a; a.svc(0x80); ISO_CHECK_EQ_UINT(one_word(a), 0xD4001001u); }
    { a64::Assembler a; a.cset(Reg::X0, Cond::Eq); ISO_CHECK_EQ_UINT(one_word(a), 0x9A9F17E0u); }
    {
        a64::Assembler a;
        auto l = a.create_label();
        a.cbz(Reg::X0, l);
        a.nop();
        a.bind(l);
        a.ret();
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0xB4000000u | (2u << 5));
    }
    {
        a64::Assembler a;
        auto l = a.create_label();
        a.cbnz(Reg::X1, l);
        a.bind(l);
        a.ret();
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0xB5000000u | (1u << 5) | 1u);
    }

    // ── division / logical / shifts / unary ────────────────────────────────
    { a64::Assembler a; a.sdiv(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC20C20u); }
    { a64::Assembler a; a.udiv(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC20820u); }
    { a64::Assembler a; a.msub(Reg::X0, Reg::X1, Reg::X2, Reg::X3); ISO_CHECK_EQ_UINT(one_word(a), 0x9B028C20u); }
    { a64::Assembler a; a.and_(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x8A020020u); }
    { a64::Assembler a; a.orr(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0xAA020020u); }
    { a64::Assembler a; a.eor(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0xCA020020u); }
    { a64::Assembler a; a.mvn(Reg::X0, Reg::X1); ISO_CHECK_EQ_UINT(one_word(a), 0xAA2103E0u); }
    { a64::Assembler a; a.lsl_reg(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC22020u); }
    { a64::Assembler a; a.lsr_reg(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC22420u); }
    { a64::Assembler a; a.asr_reg(Reg::X0, Reg::X1, Reg::X2); ISO_CHECK_EQ_UINT(one_word(a), 0x9AC22820u); }
    { a64::Assembler a; a.neg_(Reg::X0, Reg::X1); ISO_CHECK_EQ_UINT(one_word(a), 0xCB0103E0u); }

    // ── adrp placeholder ───────────────────────────────────────────────────
    {
        a64::Assembler a;
        std::size_t i0 = a.adrp_placeholder(Reg::X0);
        std::size_t i1 = a.adrp_placeholder(Reg::X1);
        auto b = a.finish();
        ISO_CHECK_EQ_UINT(word_at(b, 0), 0x90000000u);
        ISO_CHECK_EQ_UINT(word_at(b, 4), 0x90000001u);
        ISO_CHECK_EQ_UINT(i0, 0u);
        ISO_CHECK_EQ_UINT(i1, 1u);
    }

    // ── pre-indexed byte store ─────────────────────────────────────────────
    { a64::Assembler a; a.strb_pre_neg1(Reg::X4, Reg::X5); ISO_CHECK_EQ_UINT(one_word(a), 0x381FFCA4u); }
    { a64::Assembler a; a.strb_pre_neg1(Reg::X0, Reg::X0); ISO_CHECK_EQ_UINT(one_word(a), 0x381FFC00u); }

    // ── composite prologue/epilogue ────────────────────────────────────────
    {
        a64::Assembler a;
        a.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -16);
        a.add_imm(Reg::Fp, Reg::Sp, 0);
        a.movz(Reg::X0, 42, 0);
        a.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, 16);
        a.ret();
        ISO_CHECK_EQ_UINT(a.finish().size(), 20u);
    }

    return ISO_TEST_RESULT();
}
