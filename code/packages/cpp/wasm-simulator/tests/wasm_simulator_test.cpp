// Tests for the C++ wasm-simulator, using the header-only iso_test.h harness
// (pure ISO). The full-program vector mirrors the Rust crate's own test.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <vector>

#include "wasm_simulator.hpp"

namespace w = ca::wasm;

int main() {
    // ── decode ───────────────────────────────────────────────────────────
    {
        w::WasmDecoder dec;
        std::vector<std::uint8_t> code = {w::OP_I32_CONST, 42, 0, 0, 0};
        w::WasmInstruction inst = dec.decode(code, 0);
        ISO_CHECK_STR_EQ(inst.mnemonic.c_str(), "i32.const");
        ISO_CHECK(inst.operand.has_value() && *inst.operand == 42);
        ISO_CHECK_EQ_UINT(inst.size, 5u);

        // Unknown opcode / truncated code throw.
        bool threw = false;
        try {
            std::vector<std::uint8_t> bad = {0xFF};
            (void)dec.decode(bad, 0);
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── full program ─────────────────────────────────────────────────────
    {
        w::WasmSimulator sim(4);
        std::vector<std::uint8_t> program = w::assemble_wasm({
            w::encode_i32_const(1),
            w::encode_i32_const(2),
            w::encode_i32_add(),
            w::encode_local_set(0),
            w::encode_local_get(0),
            w::encode_i32_const(5),
            w::encode_i32_sub(),
            w::encode_end(),
        });
        ISO_CHECK_EQ_UINT(program.size(), 22u);

        auto traces = sim.run(program, 1000);
        ISO_CHECK_EQ_UINT(traces.size(), 8u);
        ISO_CHECK(sim.locals[0] == 3);
        ISO_CHECK_EQ_UINT(sim.stack.size(), 1u);
        ISO_CHECK(sim.stack[0] == -2);
        ISO_CHECK(static_cast<std::uint32_t>(sim.stack[0]) == 4294967294u);
        ISO_CHECK(sim.halted && sim.cycle == 8u);

        ISO_CHECK_STR_EQ(traces[0].description.c_str(), "push 1");
        ISO_CHECK(traces[0].stack_before.empty() &&
                  traces[0].stack_after.size() == 1);
        ISO_CHECK_STR_EQ(traces[2].description.c_str(), "pop 2 and 1, push 3");
        ISO_CHECK_STR_EQ(traces[6].description.c_str(), "pop 5 and 3, push -2");
        ISO_CHECK(traces[7].halted);
        ISO_CHECK_STR_EQ(traces[7].description.c_str(), "halt");
    }

    // ── stepping a halted VM throws ──────────────────────────────────────
    {
        w::WasmSimulator sim(1);
        sim.run(w::assemble_wasm({w::encode_end()}), 10);
        bool threw = false;
        try {
            (void)sim.step();
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── unknown opcode throws mid-run ────────────────────────────────────
    {
        w::WasmSimulator sim(1);
        bool threw = false;
        try {
            sim.run({0xFF}, 10);
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── stack underflow throws ───────────────────────────────────────────
    {
        w::WasmSimulator sim(1);
        bool threw = false;
        try {
            sim.run({w::OP_I32_ADD}, 10);
        } catch (const std::runtime_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── i32.add wraps modulo 2^32 ────────────────────────────────────────
    {
        w::WasmSimulator sim(0);
        auto program = w::assemble_wasm({w::encode_i32_const(2147483647),
                                         w::encode_i32_const(1),
                                         w::encode_i32_add(), w::encode_end()});
        sim.run(program, 100);
        ISO_CHECK(sim.stack[0] == (-2147483647 - 1)); // wraps to INT32_MIN
    }

    return ISO_TEST_RESULT();
}
