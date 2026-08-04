// Tests for ldp-format, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "ldp_format.hpp"

namespace ldp = ca::ldp_format;
using Bytes = std::vector<std::uint8_t>;

static ldp::LdpFile empty_file() {
    ldp::LdpFile f;
    f.header.version_major = 1;
    f.header.version_minor = 0;
    f.header.language = "twig";
    f.header.flags = 0;
    return f;
}

static ldp::LdpFile rich_file() {
    ldp::LdpFile f = empty_file();
    f.header.flags = 0b11;

    ldp::InstructionRecord i0;
    i0.instr_index = 0;
    i0.opcode = "const";
    i0.observation_count = 1000000;
    i0.observed_kind = ldp::ObservedKind::Mono;
    i0.observation_count_at_promotion = 100;
    i0.time_to_first_observation_ns = 1000;
    i0.time_to_promotion_ns = 122000000;
    i0.types_seen = {{"int", 1000000}};

    ldp::InstructionRecord i1;
    i1.instr_index = 5;
    i1.opcode = "call_builtin";
    i1.observation_count = 999999;
    i1.observed_kind = ldp::ObservedKind::Poly;
    i1.observation_count_at_promotion = 100;
    i1.time_to_first_observation_ns = 2000;
    i1.time_to_promotion_ns = 122000000;
    i1.types_seen = {{"int", 800000}, {"nil", 199999}};

    ldp::FunctionRecord fact;
    fact.name = "fact";
    fact.params = {"int"};
    fact.call_count = 1000000;
    fact.total_self_time_ns = 5000000000ull;
    fact.type_status = ldp::TypeStatus::Untyped;
    fact.promotion_state = ldp::PromotionState::JITted;
    fact.instructions = {i0, i1};

    ldp::FunctionRecord main_fn;
    main_fn.name = "main";
    main_fn.call_count = 1;
    main_fn.total_self_time_ns = 6000000000ull;
    main_fn.type_status = ldp::TypeStatus::Untyped;
    main_fn.promotion_state = ldp::PromotionState::Interp;

    ldp::ModuleRecord main_mod;
    main_mod.name = "main_mod";
    main_mod.functions = {fact, main_fn};

    ldp::FunctionRecord decode;
    decode.name = "decode";
    decode.params = {"int", "int"};  // dedup test: "int" reused
    decode.type_status = ldp::TypeStatus::PartiallyTyped;
    decode.promotion_state = ldp::PromotionState::Deopted;

    ldp::ModuleRecord another_mod;
    another_mod.name = "another_mod";
    another_mod.functions = {decode};

    f.modules = {main_mod, another_mod};
    return f;
}

int main() {
    // ── round-trips ────────────────────────────────────────────────────────
    {
        ldp::LdpFile original = empty_file();
        Bytes bytes = ldp::write(original);
        ISO_CHECK(ldp::read(bytes) == original);
    }
    {
        ldp::LdpFile original = rich_file();
        Bytes bytes = ldp::write(original);
        ISO_CHECK(ldp::read(bytes) == original);
    }

    // ── writer determinism ─────────────────────────────────────────────────
    {
        ldp::LdpFile f = rich_file();
        ISO_CHECK(ldp::write(f) == ldp::write(f));
    }

    // ── string-table dedup keeps size small ────────────────────────────────
    {
        ldp::LdpFile f = empty_file();
        const int n = 100;
        for (int i = 0; i < n; ++i) {
            ldp::InstructionRecord instr;
            instr.opcode = "const";
            instr.observation_count = 1;
            instr.observed_kind = ldp::ObservedKind::Mono;
            instr.types_seen = {{"int", 1}};
            ldp::FunctionRecord fn;
            fn.name = "fn_" + std::to_string(i);
            fn.params = {"int", "bool"};
            fn.call_count = static_cast<std::uint64_t>(i);
            fn.instructions = {instr};
            ldp::ModuleRecord m;
            m.name = "shared_module";
            m.functions = {fn};
            f.modules.push_back(m);
        }
        Bytes bytes = ldp::write(f);
        ldp::LdpFile restored = ldp::read(bytes);
        ISO_CHECK_EQ_UINT(restored.modules.size(), (unsigned)n);
        ISO_CHECK(bytes.size() / n < 150);  // dedup working
    }

    // ── reject bad magic ───────────────────────────────────────────────────
    {
        Bytes bad = {'B', 'A', 'D', 0};
        bad.resize(32, 0);
        bool threw = false;
        try {
            ldp::read(bad);
        } catch (const ldp::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == ldp::ErrorKind::BadMagic);
        }
        ISO_CHECK(threw);
    }

    // ── reject unsupported major version ───────────────────────────────────
    {
        Bytes b = {'L', 'D', 'P', 0, 99, 0, 0, 0};
        b.resize(32, 0);
        bool threw = false;
        try {
            ldp::read(b);
        } catch (const ldp::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == ldp::ErrorKind::UnsupportedMajorVersion);
        }
        ISO_CHECK(threw);
    }

    // ── reject truncated input ─────────────────────────────────────────────
    {
        Bytes bytes = ldp::write(rich_file());
        Bytes truncated(bytes.begin(), bytes.begin() + bytes.size() / 2);
        bool threw = false;
        try {
            ldp::read(truncated);
        } catch (const ldp::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == ldp::ErrorKind::UnexpectedEof);
        }
        ISO_CHECK(threw);
    }

    // ── language validation on write ───────────────────────────────────────
    {
        ldp::LdpFile f = empty_file();
        f.header.language = std::string(17, 'x');
        bool threw = false;
        try {
            ldp::write(f);
        } catch (const ldp::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == ldp::ErrorKind::LanguageTooLong);
        }
        ISO_CHECK(threw);

        f.header.language = "tw\xC9\xA1g";  // non-ASCII bytes
        threw = false;
        try {
            ldp::write(f);
        } catch (const ldp::Error& e) {
            threw = true;
            ISO_CHECK(e.kind() == ldp::ErrorKind::LanguageNotAscii);
        }
        ISO_CHECK(threw);
    }

    // ── unicode in module/function names round-trips ───────────────────────
    {
        ldp::LdpFile f = empty_file();
        ldp::FunctionRecord fn;
        fn.name = "na\xC3\xAFve_decode";  // "naïve_decode"
        fn.params = {"int"};
        ldp::ModuleRecord m;
        m.name = "\xE3\x83\xA2\xE3\x82\xB8\xE3\x83\xA5";  // katakana
        m.functions = {fn};
        f.modules = {m};
        ISO_CHECK(ldp::read(ldp::write(f)) == f);
    }

    // ── coverage: every observed_kind / type_status / promotion_state ──────
    {
        for (int k = 0; k <= 3; ++k) {
            ldp::LdpFile f = empty_file();
            ldp::InstructionRecord instr;
            instr.opcode = "const";
            instr.observation_count = 1;
            instr.observed_kind = static_cast<ldp::ObservedKind>(k);
            ldp::FunctionRecord fn;
            fn.name = "f";
            fn.call_count = 1;
            fn.instructions = {instr};
            ldp::ModuleRecord m;
            m.name = "m";
            m.functions = {fn};
            f.modules = {m};
            ldp::LdpFile restored = ldp::read(ldp::write(f));
            ISO_CHECK(restored.modules[0].functions[0].instructions[0]
                          .observed_kind == static_cast<ldp::ObservedKind>(k));
        }
        for (int ts = 0; ts <= 2; ++ts) {
            for (int ps = 0; ps <= 2; ++ps) {
                ldp::LdpFile f = empty_file();
                ldp::FunctionRecord fn;
                fn.name = "f";
                fn.type_status = static_cast<ldp::TypeStatus>(ts);
                fn.promotion_state = static_cast<ldp::PromotionState>(ps);
                ldp::ModuleRecord m;
                m.name = "m";
                m.functions = {fn};
                f.modules = {m};
                ldp::LdpFile restored = ldp::read(ldp::write(f));
                ISO_CHECK(restored.modules[0].functions[0].type_status ==
                          static_cast<ldp::TypeStatus>(ts));
                ISO_CHECK(restored.modules[0].functions[0].promotion_state ==
                          static_cast<ldp::PromotionState>(ps));
            }
        }
    }

    return ISO_TEST_RESULT();
}
