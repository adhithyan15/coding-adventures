// compiler_source_map.hpp — the source-mapping sidecar for an AOT compiler
// pipeline, in pure ISO C++17, header-only, in namespace ca::csm. A faithful
// port of the Rust `compiler-source-map` crate.
// ===========================================================================
//
// As a program is lowered source → AST → IR → (optimiser passes) → machine code,
// this sidecar records at each stage which IDs map to which, so any machine-code
// location can be traced back to its source position and vice-versa. Four
// segments (SourceToAst, AstToIr, IrToIr per pass, IrToMachineCode) plus a
// SourceMapChain that composes them for the two end-to-end queries.
//
// Value semantics throughout (mirroring the Rust structs' public fields). Where
// Rust returns `Option`, this port returns `std::optional` or a borrowed pointer
// (nullptr when absent). Lookups are linear scans — "first match wins", as in
// the Rust source.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_COMPILER_SOURCE_MAP_HPP
#define CA_COMPILER_SOURCE_MAP_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <set>
#include <string>
#include <utility>
#include <vector>

namespace ca {
namespace csm {

// A span of characters in a source file.
struct SourcePosition {
    std::string file;
    std::size_t line = 0;   // 1-based
    std::size_t column = 0; // 1-based
    std::size_t length = 0;

    bool operator==(const SourcePosition& o) const {
        return file == o.file && line == o.line && column == o.column &&
               length == o.length;
    }
    bool operator!=(const SourcePosition& o) const { return !(*this == o); }

    // "file:line:column (len=N)".
    std::string to_string() const {
        return file + ":" + std::to_string(line) + ":" + std::to_string(column) +
               " (len=" + std::to_string(length) + ")";
    }
};

// ── Segment 1: SourceToAst ───────────────────────────────────────────────────

struct SourceToAstEntry {
    SourcePosition pos;
    std::size_t ast_node_id;
};

struct SourceToAst {
    std::vector<SourceToAstEntry> entries;

    void add(SourcePosition pos, std::size_t ast_node_id) {
        entries.push_back({std::move(pos), ast_node_id});
    }
    const SourcePosition* lookup_by_node_id(std::size_t ast_node_id) const {
        for (const auto& e : entries) {
            if (e.ast_node_id == ast_node_id) return &e.pos;
        }
        return nullptr;
    }
};

// ── Segment 2: AstToIr ───────────────────────────────────────────────────────

struct AstToIrEntry {
    std::size_t ast_node_id;
    std::vector<std::int64_t> ir_ids;
};

struct AstToIr {
    std::vector<AstToIrEntry> entries;

    void add(std::size_t ast_node_id, std::vector<std::int64_t> ir_ids) {
        entries.push_back({ast_node_id, std::move(ir_ids)});
    }
    const std::vector<std::int64_t>* lookup_by_ast_node_id(
        std::size_t ast_node_id) const {
        for (const auto& e : entries) {
            if (e.ast_node_id == ast_node_id) return &e.ir_ids;
        }
        return nullptr;
    }
    std::optional<std::size_t> lookup_by_ir_id(std::int64_t ir_id) const {
        for (const auto& e : entries) {
            for (std::int64_t id : e.ir_ids) {
                if (id == ir_id) return e.ast_node_id;
            }
        }
        return std::nullopt;
    }
};

// ── Segment 3: IrToIr (one per optimiser pass) ───────────────────────────────

struct IrToIrEntry {
    std::int64_t original_id;
    std::vector<std::int64_t> new_ids;
};

struct IrToIr {
    std::vector<IrToIrEntry> entries;
    std::set<std::int64_t> deleted;
    std::string pass_name;

    explicit IrToIr(std::string name) : pass_name(std::move(name)) {}

    void add_mapping(std::int64_t original_id, std::vector<std::int64_t> new_ids) {
        entries.push_back({original_id, std::move(new_ids)});
    }
    void add_deletion(std::int64_t original_id) {
        deleted.insert(original_id);
        entries.push_back({original_id, {}});
    }
    const std::vector<std::int64_t>* lookup_by_original_id(
        std::int64_t original_id) const {
        if (deleted.count(original_id)) return nullptr;
        for (const auto& e : entries) {
            if (e.original_id == original_id) return &e.new_ids;
        }
        return nullptr;
    }
    std::optional<std::int64_t> lookup_by_new_id(std::int64_t new_id) const {
        for (const auto& e : entries) {
            for (std::int64_t id : e.new_ids) {
                if (id == new_id) return e.original_id;
            }
        }
        return std::nullopt;
    }
};

// ── Segment 4: IrToMachineCode ───────────────────────────────────────────────

struct IrToMachineCodeEntry {
    std::int64_t ir_id;
    std::size_t mc_offset;
    std::size_t mc_length;
};

struct IrToMachineCode {
    std::vector<IrToMachineCodeEntry> entries;

    void add(std::int64_t ir_id, std::size_t mc_offset, std::size_t mc_length) {
        entries.push_back({ir_id, mc_offset, mc_length});
    }
    std::optional<std::pair<std::size_t, std::size_t>> lookup_by_ir_id(
        std::int64_t ir_id) const {
        for (const auto& e : entries) {
            if (e.ir_id == ir_id) return std::make_pair(e.mc_offset, e.mc_length);
        }
        return std::nullopt;
    }
    std::optional<std::int64_t> lookup_by_mc_offset(std::size_t offset) const {
        for (const auto& e : entries) {
            if (offset >= e.mc_offset && offset < e.mc_offset + e.mc_length) {
                return e.ir_id;
            }
        }
        return std::nullopt;
    }
};

// ── SourceMapChain ───────────────────────────────────────────────────────────

struct SourceMapChain {
    SourceToAst source_to_ast;
    AstToIr ast_to_ir;
    std::vector<IrToIr> ir_to_ir;
    std::optional<IrToMachineCode> ir_to_machine_code;

    void add_optimizer_pass(IrToIr segment) {
        ir_to_ir.push_back(std::move(segment));
    }

    // Forward: source position → machine-code entries (composes all segments).
    std::optional<std::vector<IrToMachineCodeEntry>> source_to_mc(
        const SourcePosition& pos) const {
        if (!ir_to_machine_code) return std::nullopt;
        const IrToMachineCode& mc = *ir_to_machine_code;

        // Step 1: source position → AST node ID (match file + line + column).
        std::optional<std::size_t> ast_node_id;
        for (const auto& e : source_to_ast.entries) {
            if (e.pos.file == pos.file && e.pos.line == pos.line &&
                e.pos.column == pos.column) {
                ast_node_id = e.ast_node_id;
                break;
            }
        }
        if (!ast_node_id) return std::nullopt;

        // Step 2: AST node ID → IR instruction IDs.
        const std::vector<std::int64_t>* ir_ids =
            ast_to_ir.lookup_by_ast_node_id(*ast_node_id);
        if (!ir_ids) return std::nullopt;
        std::vector<std::int64_t> current = *ir_ids;

        // Step 3: follow through the optimiser passes.
        for (const IrToIr& pass : ir_to_ir) {
            std::vector<std::int64_t> next;
            for (std::int64_t id : current) {
                if (pass.deleted.count(id)) continue;
                if (const std::vector<std::int64_t>* nids =
                        pass.lookup_by_original_id(id)) {
                    next.insert(next.end(), nids->begin(), nids->end());
                }
            }
            current = std::move(next);
        }
        if (current.empty()) return std::nullopt;

        // Step 4: final IR IDs → machine code.
        std::vector<IrToMachineCodeEntry> results;
        for (std::int64_t id : current) {
            if (auto ol = mc.lookup_by_ir_id(id)) {
                results.push_back({id, ol->first, ol->second});
            }
        }
        if (results.empty()) return std::nullopt;
        return results;
    }

    // Reverse: machine-code offset → source position (nullptr if none).
    const SourcePosition* mc_to_source(std::size_t mc_offset) const {
        if (!ir_to_machine_code) return nullptr;

        auto cur = ir_to_machine_code->lookup_by_mc_offset(mc_offset);
        if (!cur) return nullptr;
        std::int64_t current_id = *cur;

        for (auto it = ir_to_ir.rbegin(); it != ir_to_ir.rend(); ++it) {
            auto orig = it->lookup_by_new_id(current_id);
            if (!orig) return nullptr;
            current_id = *orig;
        }

        auto ast_node_id = ast_to_ir.lookup_by_ir_id(current_id);
        if (!ast_node_id) return nullptr;
        return source_to_ast.lookup_by_node_id(*ast_node_id);
    }
};

}  // namespace csm
}  // namespace ca

#endif  // CA_COMPILER_SOURCE_MAP_HPP
