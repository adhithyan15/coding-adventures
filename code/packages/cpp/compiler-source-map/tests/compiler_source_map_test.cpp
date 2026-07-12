// Tests for the C++ compiler-source-map sidecar, using the header-only
// iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own unit tests.
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

#include "compiler_source_map.hpp"

namespace csm = ca::csm;

int main() {
    // ── SourcePosition display & equality ────────────────────────────────
    {
        csm::SourcePosition p{"hello.bf", 1, 3, 1};
        ISO_CHECK_STR_EQ(p.to_string().c_str(), "hello.bf:1:3 (len=1)");
        csm::SourcePosition a{"a.bf", 1, 1, 1}, b{"a.bf", 1, 1, 1},
            c{"a.bf", 2, 1, 1};
        ISO_CHECK(a == b);
        ISO_CHECK(a != c);
    }

    // ── SourceToAst ──────────────────────────────────────────────────────
    {
        csm::SourceToAst s2a;
        s2a.add({"hello.bf", 1, 1, 1}, 42);
        const csm::SourcePosition* f = s2a.lookup_by_node_id(42);
        ISO_CHECK(f != nullptr);
        if (f) ISO_CHECK_STR_EQ(f->file.c_str(), "hello.bf");
        ISO_CHECK(s2a.lookup_by_node_id(999) == nullptr);

        s2a.add({"x.bf", 1, 2, 1}, 1);
        ISO_CHECK(s2a.lookup_by_node_id(42)->column == 1);
        ISO_CHECK(s2a.lookup_by_node_id(1)->column == 2);
    }

    // ── AstToIr ──────────────────────────────────────────────────────────
    {
        csm::AstToIr a2i;
        a2i.add(42, {7, 8, 9, 10});
        const std::vector<std::int64_t>* ids = a2i.lookup_by_ast_node_id(42);
        ISO_CHECK(ids != nullptr);
        if (ids) {
            std::vector<std::int64_t> want = {7, 8, 9, 10};
            ISO_CHECK(*ids == want);
        }
        auto node = a2i.lookup_by_ir_id(8);
        ISO_CHECK(node.has_value() && *node == 42u);
        ISO_CHECK(!a2i.lookup_by_ir_id(99).has_value());
        ISO_CHECK(a2i.lookup_by_ast_node_id(0) == nullptr);
    }

    // ── IrToIr ───────────────────────────────────────────────────────────
    {
        csm::IrToIr m("contraction");
        m.add_mapping(7, {100});
        m.add_mapping(8, {100});
        m.add_mapping(9, {100});
        const std::vector<std::int64_t>* got = m.lookup_by_original_id(7);
        ISO_CHECK(got != nullptr && got->size() == 1 && (*got)[0] == 100);
        auto orig = m.lookup_by_new_id(100);
        ISO_CHECK(orig.has_value() && *orig == 7); // first one found
        ISO_CHECK_STR_EQ(m.pass_name.c_str(), "contraction");
    }
    {
        csm::IrToIr m("dead_store");
        m.add_deletion(5);
        ISO_CHECK(m.deleted.count(5) == 1);
        ISO_CHECK(m.lookup_by_original_id(5) == nullptr); // deleted
        ISO_CHECK(m.lookup_by_original_id(0) == nullptr);
        ISO_CHECK(!m.lookup_by_new_id(0).has_value());
    }

    // ── IrToMachineCode ──────────────────────────────────────────────────
    {
        csm::IrToMachineCode mc;
        mc.add(3, 0x14, 4); // bytes 0x14..0x18
        auto ol = mc.lookup_by_ir_id(3);
        ISO_CHECK(ol.has_value() && ol->first == 0x14u && ol->second == 4u);
        ISO_CHECK(mc.lookup_by_mc_offset(0x14) == std::optional<std::int64_t>(3));
        ISO_CHECK(mc.lookup_by_mc_offset(0x15).has_value()); // inside
        ISO_CHECK(mc.lookup_by_mc_offset(0x17).has_value()); // last byte
        ISO_CHECK(!mc.lookup_by_mc_offset(0x18).has_value()); // past end
        ISO_CHECK(!mc.lookup_by_ir_id(0).has_value());
    }

    // ── chain: empty / no backend ────────────────────────────────────────
    {
        csm::SourceMapChain chain;
        ISO_CHECK(chain.source_to_ast.entries.empty());
        ISO_CHECK(chain.ast_to_ir.entries.empty());
        ISO_CHECK(chain.ir_to_ir.empty());
        ISO_CHECK(!chain.ir_to_machine_code.has_value());
        ISO_CHECK(!chain.source_to_mc({"x.bf", 1, 1, 1}).has_value());
        ISO_CHECK(chain.mc_to_source(0) == nullptr);
    }

    // ── chain: full end-to-end round-trip (no passes) ────────────────────
    {
        csm::SourceMapChain chain;
        csm::SourcePosition pos{"test.bf", 1, 1, 1};
        chain.source_to_ast.add(pos, 0);
        chain.ast_to_ir.add(0, {7, 8, 9, 10});
        csm::IrToMachineCode mc;
        mc.add(7, 0, 4);
        mc.add(8, 4, 4);
        mc.add(9, 8, 4);
        mc.add(10, 12, 4);
        chain.ir_to_machine_code = std::move(mc);

        auto results = chain.source_to_mc(pos);
        ISO_CHECK(results.has_value());
        if (results) {
            ISO_CHECK_EQ_UINT(results->size(), 4u);
            ISO_CHECK_EQ_UINT((*results)[0].mc_offset, 0u);
        }
        const csm::SourcePosition* found = chain.mc_to_source(0);
        ISO_CHECK(found != nullptr);
        if (found) ISO_CHECK(*found == pos);
    }

    // ── chain: an optimiser pass is followed forward ─────────────────────
    {
        csm::SourceMapChain chain;
        csm::SourcePosition pos{"test.bf", 1, 1, 1};
        chain.source_to_ast.add(pos, 0);
        chain.ast_to_ir.add(0, {7, 8, 9});

        csm::IrToIr pass("contraction");
        pass.add_mapping(7, {100});
        pass.add_mapping(8, {100});
        pass.add_mapping(9, {100});
        chain.add_optimizer_pass(std::move(pass));

        csm::IrToMachineCode mc;
        mc.add(100, 0, 4);
        chain.ir_to_machine_code = std::move(mc);

        auto results = chain.source_to_mc(pos);
        ISO_CHECK(results.has_value());
        if (results) {
            ISO_CHECK(!results->empty());
            ISO_CHECK_EQ_UINT((*results)[0].mc_offset, 0u);
        }
    }

    // ── chain: a deleted instruction is excluded ─────────────────────────
    {
        csm::SourceMapChain chain;
        csm::SourcePosition pos{"test.bf", 1, 1, 1};
        chain.source_to_ast.add(pos, 0);
        chain.ast_to_ir.add(0, {5});

        csm::IrToIr pass("dead_store");
        pass.add_deletion(5);
        chain.add_optimizer_pass(std::move(pass));

        csm::IrToMachineCode mc;
        mc.add(5, 0, 4);
        chain.ir_to_machine_code = std::move(mc);

        ISO_CHECK(!chain.source_to_mc(pos).has_value()); // excluded
    }

    return ISO_TEST_RESULT();
}
