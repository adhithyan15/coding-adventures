// vcd_writer.hpp — a streaming Value Change Dump (VCD) writer, in pure ISO
// C++17, header-only, in namespace ca. A faithful port of the Rust `vcd-writer`
// crate.
// ===========================================================================
//
// VCD (IEEE 1364-2005 §18) is the text format every waveform viewer (GTKWave,
// Surfer, ModelSim, ...) reads. `VcdWriter` produces a complete VCD document in
// two phases: a header (open_scope / declare / close_scope / end_definitions,
// each `declare` returning a compact identifier) then a body (time / value_change).
//
// Identifiers are allocated in a bijective base-94 scheme over '!'..'~'.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_VCD_WRITER_HPP
#define CA_VCD_WRITER_HPP

#include <cstdint>
#include <optional>
#include <string>
#include <unordered_map>
#include <vector>

namespace ca {

class VcdWriter {
public:
    struct VarDef {
        std::string name;
        std::uint32_t width;
        std::string var_id;
        std::string kind;
    };

    // Create a writer with the given timescale (e.g. "1ps", "1ns").
    explicit VcdWriter(const std::string& timescale) : timescale_(timescale) {
        buf_ += "$date 2026-06-13 00:00:00 UTC $end\n";
        buf_ += "$version Silicon-Stack VCD Writer 0.1.0 $end\n";
        buf_ += "$timescale " + timescale_ + " $end\n";
    }

    // ---- header ----------------------------------------------------------
    void open_scope(const std::string& name) { open_scope_kind(name, "module"); }
    void open_scope_kind(const std::string& name, const std::string& kind) {
        buf_ += "$scope " + kind + " " + name + " $end\n";
        ++scope_depth_;
    }
    void close_scope() {
        buf_ += "$upscope $end\n";
        if (scope_depth_ > 0) {
            --scope_depth_;
        }
    }

    // Declare a variable; returns its compact VCD identifier.
    std::string declare(const std::string& name, std::uint32_t width,
                        const std::string& kind) {
        std::string id = alloc_id();
        if (width > 1) {
            buf_ += "$var " + kind + " " + std::to_string(width) + " " + id +
                    " " + name + " [" + std::to_string(width - 1) + ":0] $end\n";
        } else {
            buf_ += "$var " + kind + " " + std::to_string(width) + " " + id +
                    " " + name + " $end\n";
        }
        var_defs_.push_back(VarDef{name, width, id, kind});
        return id;
    }

    void end_definitions() {
        while (scope_depth_ > 0) {
            close_scope();
        }
        buf_ += "$enddefinitions $end\n";
        defs_ended_ = true;
    }

    // ---- body ------------------------------------------------------------
    void time(std::uint64_t t) {
        if (!defs_ended_) {
            end_definitions();
        }
        if (cur_time_ != t) {
            buf_ += "#" + std::to_string(t) + "\n";
            cur_time_ = t;
        }
    }

    void value_change(const std::string& var_id, std::int64_t value) {
        auto it = last_values_.find(var_id);
        if (it != last_values_.end() && it->second == value) {
            return;
        }
        last_values_[var_id] = value;
        buf_ += format_value_change(var_id, value);
    }

    void value_change_at(std::uint64_t t, const std::string& var_id,
                         std::int64_t value) {
        time(t);
        value_change(var_id, value);
    }

    // Emit a $dumpvars block with an initial value for every declared variable
    // (declaration order); `values` supplies overrides, others default to 0.
    void dump_initial(const std::unordered_map<std::string, std::int64_t>& values) {
        if (!cur_time_.has_value()) {
            time(0);
        }
        buf_ += "$dumpvars\n";
        for (const VarDef& d : var_defs_) {
            auto it = values.find(d.var_id);
            std::int64_t v = it != values.end() ? it->second : 0;
            buf_ += format_value_change(d.var_id, v);
            last_values_[d.var_id] = v;
        }
        buf_ += "$end\n";
    }

    // ---- output ----------------------------------------------------------
    std::string finish() { return std::move(buf_); }
    const std::string& text() const { return buf_; }

private:
    std::string timescale_;
    std::string buf_;
    std::size_t id_next_ = 0;
    bool defs_ended_ = false;
    std::optional<std::uint64_t> cur_time_;
    std::unordered_map<std::string, std::int64_t> last_values_;
    std::vector<VarDef> var_defs_;
    std::size_t scope_depth_ = 0;

    std::string alloc_id() {
        std::size_t n = id_next_++;
        std::string s;
        for (;;) {
            s.push_back(static_cast<char>('!' + static_cast<int>(n % 94)));
            n /= 94;
            if (n == 0) {
                break;
            }
            n -= 1;
        }
        return s;
    }

    const VarDef* find_def(const std::string& var_id) const {
        for (const VarDef& d : var_defs_) {
            if (d.var_id == var_id) {
                return &d;
            }
        }
        return nullptr;
    }

    std::string format_value_change(const std::string& var_id,
                                    std::int64_t value) const {
        const VarDef* def = find_def(var_id);
        if (!def) {
            return std::string();
        }
        if (def->kind == "real") {
            return "r" + std::to_string(value) + " " + var_id + "\n";
        }
        if (def->width == 1) {
            return std::string((value & 1) ? "1" : "0") + var_id + "\n";
        }
        std::uint64_t mask = def->width >= 64
                                 ? ~std::uint64_t(0)
                                 : ((std::uint64_t(1) << def->width) - 1);
        std::uint64_t masked = static_cast<std::uint64_t>(value) & mask;
        return "b" + to_binary(masked) + " " + var_id + "\n";
    }

    static std::string to_binary(std::uint64_t m) {
        if (m == 0) {
            return "0";
        }
        std::string s;
        while (m) {
            s.push_back((m & 1) ? '1' : '0');
            m >>= 1;
        }
        return std::string(s.rbegin(), s.rend());
    }
};

}  // namespace ca

#endif  // CA_VCD_WRITER_HPP
