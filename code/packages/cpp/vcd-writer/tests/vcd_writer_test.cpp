// Tests for the C++ vcd-writer, using the header-only iso_test.h harness (pure
// ISO). The main vector reproduces the Rust crate's documented example exactly.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <unordered_map>

#include "vcd_writer.hpp"

using ca::VcdWriter;

static bool contains(const std::string& hay, const std::string& needle) {
    return hay.find(needle) != std::string::npos;
}

int main() {
    // ── documented example: exact full-document match ────────────────────
    {
        VcdWriter w("1ps");
        w.open_scope("adder");
        std::string a = w.declare("a", 4, "wire");
        std::string sum = w.declare("sum", 5, "wire");
        ISO_CHECK(a == "!");
        ISO_CHECK(sum == "\"");
        w.close_scope();
        w.end_definitions();
        w.time(0);
        w.value_change(a, 0);
        w.value_change(sum, 0);
        w.time(10);
        w.value_change(a, 3);
        w.value_change(sum, 8);
        std::string want =
            "$date 2026-06-13 00:00:00 UTC $end\n"
            "$version Silicon-Stack VCD Writer 0.1.0 $end\n"
            "$timescale 1ps $end\n"
            "$scope module adder $end\n"
            "$var wire 4 ! a [3:0] $end\n"
            "$var wire 5 \" sum [4:0] $end\n"
            "$upscope $end\n"
            "$enddefinitions $end\n"
            "#0\n"
            "b0 !\n"
            "b0 \"\n"
            "#10\n"
            "b11 !\n"
            "b1000 \"\n";
        ISO_CHECK(w.text() == want);
    }

    // ── scalar / real / skip-unchanged ───────────────────────────────────
    {
        VcdWriter w("1ns");
        std::string clk = w.declare("clk", 1, "wire");
        std::string temp = w.declare("temp", 64, "real");
        w.end_definitions();
        w.value_change_at(0, clk, 0);
        w.value_change_at(0, temp, 42);
        w.value_change_at(5, clk, 1);
        ISO_CHECK(contains(w.text(), "\n0" + clk + "\n"));
        ISO_CHECK(contains(w.text(), "\n1" + clk + "\n"));
        ISO_CHECK(contains(w.text(), "r42 " + temp + "\n"));
    }
    {
        VcdWriter w("1ps");
        std::string x = w.declare("x", 4, "wire");
        w.end_definitions();
        w.time(0);
        w.value_change(x, 5);
        w.value_change(x, 5);  // skipped
        std::string txt = w.text();
        std::size_t count = 0, pos = 0;
        while ((pos = txt.find("b101 " + x, pos)) != std::string::npos) {
            ++count;
            ++pos;
        }
        ISO_CHECK_EQ_UINT((unsigned)count, 1u);
    }

    // ── dump_initial ─────────────────────────────────────────────────────
    {
        VcdWriter w("1ps");
        std::string a = w.declare("a", 4, "wire");
        std::string b = w.declare("b", 4, "wire");
        w.end_definitions();
        std::unordered_map<std::string, std::int64_t> init;
        init[a] = 7;
        w.dump_initial(init);
        ISO_CHECK(contains(w.text(), "$dumpvars\n"));
        ISO_CHECK(contains(w.text(), "b111 " + a + "\n"));  // override
        ISO_CHECK(contains(w.text(), "b0 " + b + "\n"));    // default 0
    }

    // ── two-character identifiers ─────────────────────────────────────────
    {
        VcdWriter w("1ps");
        std::string id;
        for (int i = 0; i < 94; ++i) {
            id = w.declare("s", 1, "wire");
        }
        id = w.declare("s", 1, "wire");
        ISO_CHECK(id == "!!");
    }

    // ── finish() returns the buffer ──────────────────────────────────────
    {
        VcdWriter w("1ps");
        w.declare("c", 1, "wire");
        std::string out = w.finish();
        ISO_CHECK(contains(out, "$timescale 1ps $end\n"));
    }

    return ISO_TEST_RESULT();
}
