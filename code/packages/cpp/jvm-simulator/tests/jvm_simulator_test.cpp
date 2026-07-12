// Tests for the C++ jvm-simulator, using the header-only iso_test.h harness
// (pure ISO). The program vectors mirror the Rust crate's own tests; the error
// cases exercise the exceptions that replace the Rust panics.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <vector>

#include "jvm_simulator.hpp"

namespace j = ca::jvm;

int main() {
    // ── basic program: x = 1 + 2; return x  (returns 3) ──────────────────
    {
        j::JVMSimulator sim;
        auto prog = j::assemble_jvm({
            {j::OP_ICONST_0 + 1, {}},  // iconst_1
            {j::OP_ICONST_0 + 2, {}},  // iconst_2
            {j::OP_IADD, {}},
            {j::OP_ISTORE_0, {}},
            {j::OP_ILOAD_0, {}},
            {j::OP_IRETURN, {}},
        });
        ISO_CHECK_EQ_UINT(prog.size(), 6u);
        sim.load(prog, {}, 16);
        auto traces = sim.run(100);
        ISO_CHECK_EQ_UINT(traces.size(), 6u);
        ISO_CHECK(sim.return_value.has_value() && *sim.return_value == 3);
        ISO_CHECK(sim.halted);

        ISO_CHECK_STR_EQ(traces[0].opcode.c_str(), "iconst_1");
        ISO_CHECK_STR_EQ(traces[0].description.c_str(), "push 1");
        ISO_CHECK(traces[0].stack_before.empty() &&
                  traces[0].stack_after.size() == 1);
        ISO_CHECK(traces[2].stack_after[0] == 3);  // after iadd
        ISO_CHECK_STR_EQ(traces[3].description.c_str(),
                         "pop 3, store in locals[0]");
        ISO_CHECK(traces[3].locals_snapshot[0].has_value() &&
                  *traces[3].locals_snapshot[0] == 3);
        ISO_CHECK_STR_EQ(traces[5].description.c_str(), "return 3");
    }

    // ── ldc + bipush with a negative value: -42 - 100 = -142 ──────────────
    {
        j::JVMSimulator sim;
        auto prog = j::assemble_jvm({
            {j::OP_BIPUSH, {-42}},
            {j::OP_LDC, {0}},
            {j::OP_ISUB, {}},
            {j::OP_IRETURN, {}},
        });
        sim.load(prog, {100}, 16);
        sim.run(100);
        ISO_CHECK(sim.return_value.has_value() && *sim.return_value == -142);
    }

    // ── if_icmpeq branch taken: 5 == 5 jumps over iconst_1 to iconst_4 ────
    {
        j::JVMSimulator sim;
        auto prog = j::assemble_jvm({
            {j::OP_ICONST_0 + 5, {}},  // push 5
            {j::OP_ICONST_0 + 5, {}},  // push 5
            {j::OP_IF_ICMPEQ, {4}},    // +4 -> target
            {j::OP_ICONST_0 + 1, {}},  // skipped
            {j::OP_ICONST_0 + 4, {}},  // target: push 4
            {j::OP_IRETURN, {}},
        });
        sim.load(prog, {}, 16);
        auto traces = sim.run(10);
        ISO_CHECK(sim.return_value.has_value() && *sim.return_value == 4);
        bool found = false;
        for (const auto& t : traces)
            if (t.opcode == "if_icmpeq") found = true;
        ISO_CHECK(found);
    }

    // ── idiv INT32_MIN / -1 wraps to INT32_MIN (no UB) ───────────────────
    {
        j::JVMSimulator sim;
        auto prog = j::assemble_jvm({
            {j::OP_LDC, {0}},  // INT32_MIN
            {j::OP_LDC, {1}},  // -1
            {j::OP_IDIV, {}},
            {j::OP_IRETURN, {}},
        });
        sim.load(prog, {INT32_MIN, -1}, 16);
        sim.run(10);
        ISO_CHECK(sim.return_value.has_value() &&
                  *sim.return_value == INT32_MIN);
    }

    // ── iadd wraps modulo 2^32 ───────────────────────────────────────────
    {
        j::JVMSimulator sim;
        auto prog = j::assemble_jvm({
            {j::OP_LDC, {0}},  // INT32_MAX
            {j::OP_ICONST_0 + 1, {}},
            {j::OP_IADD, {}},
            {j::OP_IRETURN, {}},
        });
        sim.load(prog, {INT32_MAX}, 16);
        sim.run(10);
        ISO_CHECK(sim.return_value.has_value() &&
                  *sim.return_value == INT32_MIN);  // MAX + 1 wraps
    }

    // ── division by zero throws (the Rust panic) ─────────────────────────
    {
        j::JVMSimulator sim;
        auto prog = j::assemble_jvm({
            {j::OP_ICONST_0 + 5, {}},
            {j::OP_ICONST_0, {}},  // push 0
            {j::OP_IDIV, {}},
        });
        sim.load(prog, {}, 16);
        bool threw = false;
        try {
            sim.run(10);
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── stepping a halted VM throws (the Rust panic) ─────────────────────
    {
        j::JVMSimulator sim;
        sim.load(j::assemble_jvm({{j::OP_RETURN, {}}}), {}, 16);
        auto traces = sim.run(10);
        ISO_CHECK_EQ_UINT(traces.size(), 1u);
        bool threw = false;
        try {
            (void)sim.step();
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── an uninitialized local load throws ───────────────────────────────
    {
        j::JVMSimulator sim;
        sim.load(j::assemble_jvm({{j::OP_ILOAD_0, {}}}), {}, 16);
        bool threw = false;
        try {
            sim.run(10);
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── an unknown opcode throws ─────────────────────────────────────────
    {
        j::JVMSimulator sim;
        sim.load({0xEE}, {}, 16);
        bool threw = false;
        try {
            sim.run(10);
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── encode helpers pick compact vs wide forms ────────────────────────
    {
        ISO_CHECK(j::encode_iconst(3) == std::vector<std::uint8_t>{
                                             static_cast<std::uint8_t>(
                                                 j::OP_ICONST_0 + 3)});
        ISO_CHECK((j::encode_iconst(-42) ==
                   std::vector<std::uint8_t>{j::OP_BIPUSH, 0xD6}));
        ISO_CHECK(j::encode_iload(0) ==
                  std::vector<std::uint8_t>{j::OP_ILOAD_0});
        ISO_CHECK((j::encode_iload(7) ==
                   std::vector<std::uint8_t>{j::OP_ILOAD, 7}));
        ISO_CHECK(j::encode_istore(2) ==
                  std::vector<std::uint8_t>{static_cast<std::uint8_t>(
                      j::OP_ISTORE_0 + 2)});
    }

    return ISO_TEST_RESULT();
}
