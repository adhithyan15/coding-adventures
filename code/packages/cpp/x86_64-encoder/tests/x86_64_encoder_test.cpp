// Tests for x86_64-encoder, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests (byte-exact x86-64 encodings).
#include "iso_test.h"

#include <cstdint>
#include <vector>

#include "x86_64_encoder.hpp"

namespace x64 = ca::x86_64_encoder;
using Reg = x64::Reg;
using Cond = x64::Cond;
using Bytes = std::vector<std::uint8_t>;

static Bytes fin(x64::Assembler& a) { return a.finish(); }

int main() {
    // ── MOV ────────────────────────────────────────────────────────────────
    { x64::Assembler a; a.mov_r64_r64(Reg::Rax, Reg::Rdi); ISO_CHECK((fin(a) == Bytes{0x48, 0x89, 0xF8})); }
    { x64::Assembler a; a.mov_r64_r64(Reg::R15, Reg::R8); ISO_CHECK((fin(a) == Bytes{0x4D, 0x89, 0xC7})); }
    { x64::Assembler a; a.mov_r64_imm32(Reg::Rax, 42); ISO_CHECK((fin(a) == Bytes{0x48, 0xC7, 0xC0, 0x2A, 0x00, 0x00, 0x00})); }
    { x64::Assembler a; a.mov_r64_imm32(Reg::Rax, -1); ISO_CHECK((fin(a) == Bytes{0x48, 0xC7, 0xC0, 0xFF, 0xFF, 0xFF, 0xFF})); }
    { x64::Assembler a; a.mov_r64_imm64(Reg::Rax, 0x1234567890ABCDEFull); ISO_CHECK((fin(a) == Bytes{0x48, 0xB8, 0xEF, 0xCD, 0xAB, 0x90, 0x78, 0x56, 0x34, 0x12})); }
    { x64::Assembler a; a.mov_r64_imm64(Reg::R10, 0xDEADBEEFCAFEBABEull); ISO_CHECK((fin(a) == Bytes{0x49, 0xBA, 0xBE, 0xBA, 0xFE, 0xCA, 0xEF, 0xBE, 0xAD, 0xDE})); }

    // ── memory ─────────────────────────────────────────────────────────────
    { x64::Assembler a; a.mov_r64_mem(Reg::Rax, Reg::Rbp, -8); ISO_CHECK((fin(a) == Bytes{0x48, 0x8B, 0x85, 0xF8, 0xFF, 0xFF, 0xFF})); }
    { x64::Assembler a; a.mov_mem_r64(Reg::Rbp, -8, Reg::Rdi); ISO_CHECK((fin(a) == Bytes{0x48, 0x89, 0xBD, 0xF8, 0xFF, 0xFF, 0xFF})); }
    { x64::Assembler a; a.mov_r64_mem(Reg::Rax, Reg::Rsp, -8); ISO_CHECK((fin(a) == Bytes{0x48, 0x8B, 0x84, 0x24, 0xF8, 0xFF, 0xFF, 0xFF})); }

    // ── arithmetic ─────────────────────────────────────────────────────────
    { x64::Assembler a; a.add(Reg::Rax, Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x48, 0x01, 0xC8})); }
    { x64::Assembler a; a.sub(Reg::Rax, Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x48, 0x29, 0xC8})); }
    { x64::Assembler a; a.imul(Reg::Rax, Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x48, 0x0F, 0xAF, 0xC1})); }
    { x64::Assembler a; a.add_imm32(Reg::Rax, 1000); ISO_CHECK((fin(a) == Bytes{0x48, 0x81, 0xC0, 0xE8, 0x03, 0x00, 0x00})); }
    { x64::Assembler a; a.neg_(Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x48, 0xF7, 0xD8})); }
    { x64::Assembler a; a.idiv(Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x48, 0xF7, 0xF9})); }
    { x64::Assembler a; a.div(Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x48, 0xF7, 0xF1})); }
    { x64::Assembler a; a.cqo(); ISO_CHECK((fin(a) == Bytes{0x48, 0x99})); }

    // ── logical ────────────────────────────────────────────────────────────
    {
        x64::Assembler a;
        a.and_(Reg::Rax, Reg::Rcx);
        a.or_(Reg::Rax, Reg::Rcx);
        a.xor_(Reg::Rax, Reg::Rcx);
        a.test_(Reg::Rax, Reg::Rcx);
        a.not_(Reg::Rax);
        ISO_CHECK((fin(a) == Bytes{0x48, 0x21, 0xC8, 0x48, 0x09, 0xC8, 0x48,
                                   0x31, 0xC8, 0x48, 0x85, 0xC8, 0x48, 0xF7,
                                   0xD0}));
    }

    // ── shifts ─────────────────────────────────────────────────────────────
    { x64::Assembler a; a.shl_cl(Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x48, 0xD3, 0xE0})); }
    { x64::Assembler a; a.shr_cl(Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x48, 0xD3, 0xE8})); }
    { x64::Assembler a; a.sar_cl(Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x48, 0xD3, 0xF8})); }
    { x64::Assembler a; a.shl_imm8(Reg::Rax, 3); ISO_CHECK((fin(a) == Bytes{0x48, 0xC1, 0xE0, 0x03})); }

    // ── compare + set ──────────────────────────────────────────────────────
    { x64::Assembler a; a.cmp(Reg::Rax, Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x48, 0x39, 0xC8})); }
    { x64::Assembler a; a.setcc(Cond::E, Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x40, 0x0F, 0x94, 0xC0})); }
    { x64::Assembler a; a.movzx_r64_r8(Reg::Rax, Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x48, 0x0F, 0xB6, 0xC0})); }

    // ── SSE2 scalar double ─────────────────────────────────────────────────
    {
        x64::Assembler a;
        a.movsd_load(Reg::Rax, Reg::Rbp, 8);
        a.movsd_store(Reg::Rbp, 8, Reg::Rax);
        ISO_CHECK((fin(a) == Bytes{0xF2, 0x0F, 0x10, 0x85, 0x08, 0x00, 0x00,
                                   0x00, 0xF2, 0x0F, 0x11, 0x85, 0x08, 0x00,
                                   0x00, 0x00}));
    }
    {
        x64::Assembler a;
        a.addsd(Reg::Rax, Reg::Rcx);
        a.subsd(Reg::Rax, Reg::Rcx);
        a.mulsd(Reg::Rax, Reg::Rcx);
        a.divsd(Reg::Rax, Reg::Rcx);
        ISO_CHECK((fin(a) == Bytes{0xF2, 0x0F, 0x58, 0xC1, 0xF2, 0x0F, 0x5C,
                                   0xC1, 0xF2, 0x0F, 0x59, 0xC1, 0xF2, 0x0F,
                                   0x5E, 0xC1}));
    }
    { x64::Assembler a; a.ucomisd(Reg::Rax, Reg::Rcx); ISO_CHECK((fin(a) == Bytes{0x66, 0x0F, 0x2E, 0xC1})); }
    { x64::Assembler a; a.sqrtsd(Reg::Rax, Reg::Rax); ISO_CHECK((fin(a) == Bytes{0xF2, 0x0F, 0x51, 0xC0})); }

    // ── int ⇄ real conversions ─────────────────────────────────────────────
    {
        x64::Assembler a;
        a.cvtsi2sd(Reg::Rax, Reg::Rax);
        a.cvttsd2si(Reg::Rax, Reg::Rax);
        a.roundsd(Reg::Rax, Reg::Rax, 1);
        ISO_CHECK((fin(a) == Bytes{0xF2, 0x48, 0x0F, 0x2A, 0xC0, 0xF2, 0x48,
                                   0x0F, 0x2C, 0xC0, 0x66, 0x0F, 0x3A, 0x0B,
                                   0xC0, 0x01}));
    }
    {
        x64::Assembler a;
        a.cvtsi2sd(Reg::Rcx, Reg::R8);
        a.cvttsd2si(Reg::R8, Reg::Rcx);
        ISO_CHECK((fin(a) == Bytes{0xF2, 0x49, 0x0F, 0x2A, 0xC8, 0xF2, 0x4C,
                                   0x0F, 0x2C, 0xC1}));
    }

    // ── stack ──────────────────────────────────────────────────────────────
    { x64::Assembler a; a.push(Reg::Rbp); a.pop(Reg::Rbp); ISO_CHECK((fin(a) == Bytes{0x55, 0x5D})); }
    { x64::Assembler a; a.push(Reg::R15); ISO_CHECK((fin(a) == Bytes{0x41, 0x57})); }

    // ── control flow ───────────────────────────────────────────────────────
    {
        x64::Assembler a;
        auto fwd = a.create_label();
        a.jmp(fwd);
        a.nop();
        a.nop();
        a.bind(fwd);
        a.ret();
        ISO_CHECK((fin(a) == Bytes{0xE9, 0x02, 0x00, 0x00, 0x00, 0x90, 0x90,
                                   0xC3}));
    }
    {
        x64::Assembler a;
        auto top = a.create_label();
        a.bind(top);
        a.nop();
        a.jcc(Cond::Ne, top);
        Bytes b = fin(a);
        ISO_CHECK(b[0] == 0x90 && b[1] == 0x0F && b[2] == 0x85);
        std::int32_t disp = static_cast<std::int32_t>(
            static_cast<std::uint32_t>(b[3]) |
            (static_cast<std::uint32_t>(b[4]) << 8) |
            (static_cast<std::uint32_t>(b[5]) << 16) |
            (static_cast<std::uint32_t>(b[6]) << 24));
        ISO_CHECK_EQ_INT(disp, -7);
    }
    {
        x64::Assembler a;
        auto l = a.create_label();
        a.jmp(l);
        bool threw = false;
        try { a.finish(); } catch (const x64::Error& e) { threw = true; ISO_CHECK(e.kind() == x64::ErrorKind::UnboundLabel); }
        ISO_CHECK(threw);
    }
    {
        x64::Assembler a;
        auto l = a.create_label();
        a.bind(l);
        bool threw = false;
        try { a.bind(l); } catch (const x64::Error& e) { threw = true; ISO_CHECK(e.kind() == x64::ErrorKind::LabelAlreadyBound); }
        ISO_CHECK(threw);
    }

    // ── calls / relocs ─────────────────────────────────────────────────────
    {
        x64::Assembler a;
        a.call_rel32("__twig_print_i64", x64::ExternalRelocKind::PltRel32);
        auto relocs = a.external_relocs();
        ISO_CHECK((fin(a) == Bytes{0xE8, 0x00, 0x00, 0x00, 0x00}));
        ISO_CHECK_EQ_UINT(relocs.size(), 1u);
        ISO_CHECK(relocs[0].symbol == "__twig_print_i64");
        ISO_CHECK(relocs[0].kind == x64::ExternalRelocKind::PltRel32);
        ISO_CHECK_EQ_INT(relocs[0].addend, -4);
        ISO_CHECK_EQ_UINT(relocs[0].patch_offset, 1u);
    }
    {
        x64::Assembler a;
        auto top = a.create_label();
        a.bind(top);
        a.nop();
        a.call_label(top);
        Bytes b = fin(a);
        ISO_CHECK(b[0] == 0x90 && b[1] == 0xE8);
        std::int32_t disp = static_cast<std::int32_t>(
            static_cast<std::uint32_t>(b[2]) |
            (static_cast<std::uint32_t>(b[3]) << 8) |
            (static_cast<std::uint32_t>(b[4]) << 16) |
            (static_cast<std::uint32_t>(b[5]) << 24));
        ISO_CHECK_EQ_INT(disp, -6);
    }
    { x64::Assembler a; a.call_r64(Reg::Rax); ISO_CHECK((fin(a) == Bytes{0x40, 0xFF, 0xD0})); }
    { x64::Assembler a; a.ret(); ISO_CHECK((fin(a) == Bytes{0xC3})); }
    { x64::Assembler a; a.ud2(); ISO_CHECK((fin(a) == Bytes{0x0F, 0x0B})); }
    {
        x64::Assembler a;
        a.lea_rip_rel(Reg::Rax, "_twig_globals", x64::ExternalRelocKind::PcRel32);
        auto relocs = a.external_relocs();
        ISO_CHECK((fin(a) == Bytes{0x48, 0x8D, 0x05, 0x00, 0x00, 0x00, 0x00}));
        ISO_CHECK_EQ_UINT(relocs.size(), 1u);
        ISO_CHECK(relocs[0].symbol == "_twig_globals");
        ISO_CHECK_EQ_UINT(relocs[0].patch_offset, 3u);
    }

    // ── worked examples ────────────────────────────────────────────────────
    {
        x64::Assembler a;
        a.mov_r64_r64(Reg::Rax, Reg::Rdi);
        a.add(Reg::Rax, Reg::Rsi);
        a.ret();
        ISO_CHECK((fin(a) == Bytes{0x48, 0x89, 0xF8, 0x48, 0x01, 0xF0, 0xC3}));
    }
    {
        x64::Assembler a;
        a.mov_r64_r64(Reg::Rax, Reg::Rcx);
        a.add(Reg::Rax, Reg::Rdx);
        a.ret();
        ISO_CHECK((fin(a) == Bytes{0x48, 0x89, 0xC8, 0x48, 0x01, 0xD0, 0xC3}));
    }

    return ISO_TEST_RESULT();
}
