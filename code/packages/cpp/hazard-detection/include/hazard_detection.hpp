// hazard_detection.hpp — pipeline hazard detection for a 5-stage CPU,
// header-only in pure ISO C++17 (namespace ca::hazard_detection). A faithful
// port of the Rust `hazard-detection` crate.
// ===========================================================================
//
// Detects data, control, and structural hazards in a classic in-order 5-stage
// pipeline and decides the action: forward, stall, or flush. A `PipelineSlot`
// is an ISA-independent snapshot of the instruction in a stage; each detector
// returns a `HazardResult` (action + optional forwarded value + stall/flush
// counts + a human-readable reason).
//
// Priority (most severe wins): Flush > Stall > ForwardFromEX > ForwardFromMEM >
// None.
//
// DIVERGENCE FROM RUST. `Option` -> `std::optional`; `Vec` / `String` ->
// `std::vector` / `std::string`. The `HazardUnit`'s Option<&PipelineSlot>
// arguments become nullable const pointers.
//
// Pure ISO C++17: compiles under GCC, Clang and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors; no <cmath>, no compiler extensions.
#ifndef CA_HAZARD_DETECTION_HPP
#define CA_HAZARD_DETECTION_HPP

#include <cstdint>
#include <cstdio>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace hazard_detection {

// The action the hazard unit tells the pipeline to take.
enum class HazardAction { None, ForwardFromMEM, ForwardFromEX, Stall, Flush };

// Numeric priority (higher = more severe).
inline std::uint8_t priority(HazardAction a) {
    switch (a) {
        case HazardAction::None: return 0;
        case HazardAction::ForwardFromMEM: return 1;
        case HazardAction::ForwardFromEX: return 2;
        case HazardAction::Stall: return 3;
        case HazardAction::Flush: return 4;
    }
    return 0;
}

// An ISA-independent snapshot of one pipeline stage.
struct PipelineSlot {
    bool valid = false;
    std::uint32_t pc = 0;
    std::vector<std::uint32_t> source_regs;
    std::optional<std::uint32_t> dest_reg;
    std::optional<std::int64_t> dest_value;
    bool is_branch = false;
    bool branch_taken = false;
    bool branch_predicted_taken = false;
    bool mem_read = false;
    bool mem_write = false;
    bool uses_alu = false;
    bool uses_fp = false;
};

// The outcome of hazard detection.
struct HazardResult {
    HazardAction action = HazardAction::None;
    std::optional<std::int64_t> forwarded_value;
    std::string forwarded_from;
    std::uint32_t stall_cycles = 0;
    std::uint32_t flush_count = 0;
    std::string reason;

    HazardResult() = default;
    HazardResult(HazardAction a, const std::string& why) : action(a), reason(why) {}
};

namespace detail {
// printf-style formatting into a std::string (reasons are short and bounded).
template <typename... A>
inline std::string fmt(const char* f, A... a) {
    char buf[256];
    std::snprintf(buf, sizeof(buf), f, a...);
    return std::string(buf);
}
}  // namespace detail

// Whichever result is more severe (ties keep `a`).
inline HazardResult pick_higher_priority(HazardResult a, HazardResult b) {
    return priority(b.action) > priority(a.action) ? b : a;
}

// ── Data hazard detector ─────────────────────────────────────────────────────
class DataHazardDetector {
public:
    HazardResult detect(const PipelineSlot& id, const PipelineSlot& ex,
                        const PipelineSlot& mem) const {
        if (!id.valid) return {HazardAction::None, "ID stage is empty (bubble)"};
        if (id.source_regs.empty())
            return {HazardAction::None, "instruction has no source registers"};

        HazardResult worst{HazardAction::None, "no data dependencies detected"};
        for (std::uint32_t src : id.source_regs)
            worst = pick_higher_priority(worst, check_single(src, ex, mem));
        return worst;
    }

private:
    HazardResult check_single(std::uint32_t src, const PipelineSlot& ex,
                              const PipelineSlot& mem) const {
        if (ex.valid && ex.dest_reg && *ex.dest_reg == src) {
            if (ex.mem_read) {
                HazardResult r{HazardAction::Stall, ""};
                r.stall_cycles = 1;
                r.reason = detail::fmt(
                    "load-use hazard: R%u is being loaded by instruction at "
                    "PC=0x%04X -- must stall 1 cycle",
                    src, static_cast<unsigned>(ex.pc));
                return r;
            }
            HazardResult r{HazardAction::ForwardFromEX, ""};
            r.forwarded_value = ex.dest_value;
            r.forwarded_from = "EX";
            r.reason = detail::fmt(
                "RAW hazard on R%u: forwarding from EX stage (instruction at "
                "PC=0x%04X)",
                src, static_cast<unsigned>(ex.pc));
            return r;
        }
        if (mem.valid && mem.dest_reg && *mem.dest_reg == src) {
            HazardResult r{HazardAction::ForwardFromMEM, ""};
            r.forwarded_value = mem.dest_value;
            r.forwarded_from = "MEM";
            r.reason = detail::fmt(
                "RAW hazard on R%u: forwarding from MEM stage (instruction at "
                "PC=0x%04X)",
                src, static_cast<unsigned>(mem.pc));
            return r;
        }
        return {HazardAction::None,
                detail::fmt("R%u has no pending writes in EX or MEM", src)};
    }
};

// ── Control hazard detector ──────────────────────────────────────────────────
class ControlHazardDetector {
public:
    HazardResult detect(const PipelineSlot& ex) const {
        if (!ex.valid) return {HazardAction::None, "EX stage is empty (bubble)"};
        if (!ex.is_branch)
            return {HazardAction::None, "EX stage instruction is not a branch"};

        if (ex.branch_predicted_taken == ex.branch_taken) {
            return {HazardAction::None,
                    detail::fmt("branch at PC=0x%04X correctly predicted %s",
                                static_cast<unsigned>(ex.pc),
                                ex.branch_taken ? "taken" : "not taken")};
        }
        const char* dir = ex.branch_taken
                              ? "predicted not-taken, actually taken"
                              : "predicted taken, actually not-taken";
        HazardResult r{HazardAction::Flush, ""};
        r.flush_count = 2;
        r.reason = detail::fmt(
            "branch misprediction at PC=0x%04X: %s -- flushing IF and ID stages",
            static_cast<unsigned>(ex.pc), dir);
        return r;
    }
};

// ── Structural hazard detector ───────────────────────────────────────────────
class StructuralHazardDetector {
public:
    StructuralHazardDetector(std::uint32_t num_alus, std::uint32_t num_fp_units,
                             bool split_caches)
        : num_alus_(num_alus), num_fp_units_(num_fp_units), split_caches_(split_caches) {}

    HazardResult detect(const PipelineSlot& id, const PipelineSlot& ex,
                        const PipelineSlot* if_stage,
                        const PipelineSlot* mem_stage) const {
        HazardResult exec = exec_conflict(id, ex);
        if (exec.action != HazardAction::None) return exec;
        if (if_stage != nullptr && mem_stage != nullptr) {
            HazardResult mem = mem_conflict(*if_stage, *mem_stage);
            if (mem.action != HazardAction::None) return mem;
        }
        return {HazardAction::None,
                "no structural hazards -- all resources available"};
    }

private:
    std::uint32_t num_alus_, num_fp_units_;
    bool split_caches_;

    HazardResult exec_conflict(const PipelineSlot& id, const PipelineSlot& ex) const {
        if (!id.valid || !ex.valid)
            return {HazardAction::None, "one or both stages are empty (bubble)"};
        if (id.uses_alu && ex.uses_alu && num_alus_ < 2) {
            HazardResult r{HazardAction::Stall, ""};
            r.stall_cycles = 1;
            r.reason = detail::fmt(
                "structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need "
                "the ALU, but only %u ALU available",
                static_cast<unsigned>(id.pc), static_cast<unsigned>(ex.pc), num_alus_);
            return r;
        }
        if (id.uses_fp && ex.uses_fp && num_fp_units_ < 2) {
            HazardResult r{HazardAction::Stall, ""};
            r.stall_cycles = 1;
            r.reason = detail::fmt(
                "structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) need "
                "the FP unit, but only %u FP unit available",
                static_cast<unsigned>(id.pc), static_cast<unsigned>(ex.pc), num_fp_units_);
            return r;
        }
        return {HazardAction::None, "no execution unit conflict"};
    }

    HazardResult mem_conflict(const PipelineSlot& if_stage,
                              const PipelineSlot& mem_stage) const {
        if (split_caches_)
            return {HazardAction::None, "split caches -- no memory port conflict"};
        if (if_stage.valid && mem_stage.valid &&
            (mem_stage.mem_read || mem_stage.mem_write)) {
            HazardResult r{HazardAction::Stall, ""};
            r.stall_cycles = 1;
            r.reason = detail::fmt(
                "structural hazard: IF (fetch at PC=0x%04X) and MEM (%s at "
                "PC=0x%04X) both need the shared memory bus",
                static_cast<unsigned>(if_stage.pc),
                mem_stage.mem_read ? "load" : "store",
                static_cast<unsigned>(mem_stage.pc));
            return r;
        }
        return {HazardAction::None, "no memory port conflict"};
    }
};

// ── Combined unit ────────────────────────────────────────────────────────────
class HazardUnit {
public:
    HazardUnit(std::uint32_t num_alus, std::uint32_t num_fp_units, bool split_caches)
        : structural_(num_alus, num_fp_units, split_caches) {}

    HazardResult check(const PipelineSlot& if_stage, const PipelineSlot& id,
                       const PipelineSlot& ex, const PipelineSlot& mem) {
        HazardResult control = control_.detect(ex);
        HazardResult data = data_.detect(id, ex, mem);
        HazardResult structural = structural_.detect(id, ex, &if_stage, &mem);
        HazardResult best = pick_higher_priority(control, data);
        best = pick_higher_priority(best, structural);
        history_.push_back(best);
        return best;
    }

    const std::vector<HazardResult>& history() const { return history_; }

    std::uint32_t stall_count() const {
        std::uint32_t total = 0;
        for (const auto& r : history_) total += r.stall_cycles;
        return total;
    }
    std::uint32_t flush_count() const {
        std::uint32_t c = 0;
        for (const auto& r : history_)
            if (r.action == HazardAction::Flush) c++;
        return c;
    }
    std::uint32_t forward_count() const {
        std::uint32_t c = 0;
        for (const auto& r : history_)
            if (r.action == HazardAction::ForwardFromEX ||
                r.action == HazardAction::ForwardFromMEM)
                c++;
        return c;
    }

private:
    DataHazardDetector data_;
    ControlHazardDetector control_;
    StructuralHazardDetector structural_;
    std::vector<HazardResult> history_;
};

}  // namespace hazard_detection
}  // namespace ca

#endif  // CA_HAZARD_DETECTION_HPP
