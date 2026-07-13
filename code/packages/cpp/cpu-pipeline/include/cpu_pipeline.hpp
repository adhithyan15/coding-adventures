// cpu_pipeline.hpp — Configurable N-stage CPU instruction pipeline, C++17.
// ============================================================================
//
// A faithful, header-only port of the Rust `cpu-pipeline` crate, in namespace
// `ca::cpu_pipeline`. It manages the FLOW of instructions through pipeline
// stages (IF → ID → EX → MEM → WB and deeper variants); the ISA work is
// injected via `std::function` callbacks. It handles normal advancement,
// stalls (freeze + bubble), flushes (discard speculative work), forwarding, and
// statistics (IPC/CPI, stall/flush/bubble cycles).
//
// This port mirrors the Rust structure directly: `PipelineToken` carries a
// `std::unordered_map<std::string,int64_t>` of stage-entry cycles, callbacks are
// `std::function`, and pipeline slots are `std::optional<PipelineToken>`. Where
// the Rust `Pipeline::new` returns `Result`, this port throws
// `std::invalid_argument`. Pure ISO C++17.

#ifndef CPU_PIPELINE_HPP
#define CPU_PIPELINE_HPP

#include <cstdint>
#include <functional>
#include <optional>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

namespace ca {
namespace cpu_pipeline {

// ── Stage category ───────────────────────────────────────────────────────────
enum class StageCategory { Fetch, Decode, Execute, Memory, Writeback };

inline std::string to_string(StageCategory c) {
    switch (c) {
    case StageCategory::Fetch:
        return "fetch";
    case StageCategory::Decode:
        return "decode";
    case StageCategory::Execute:
        return "execute";
    case StageCategory::Memory:
        return "memory";
    case StageCategory::Writeback:
        return "writeback";
    }
    return "unknown";
}

// ── Pipeline stage definition ────────────────────────────────────────────────
struct PipelineStage {
    std::string name;
    std::string description;
    StageCategory category;
};

// ── Pipeline token — one instruction flowing through the pipeline ────────────
struct PipelineToken {
    std::int64_t pc = 0;
    std::int64_t raw_instruction = 0;
    std::string opcode;

    std::int64_t rs1 = -1, rs2 = -1, rd = -1, immediate = 0;

    bool reg_write = false, mem_read = false, mem_write = false,
         is_branch = false, is_halt = false;

    std::int64_t alu_result = 0, mem_data = 0, write_data = 0, branch_target = 0;
    bool branch_taken = false;

    bool is_bubble = false;
    std::unordered_map<std::string, std::int64_t> stage_entered;
    std::string forwarded_from;

    // A fresh (non-bubble) token: registers -1, all signals clear (defaults).
    static PipelineToken make() { return PipelineToken{}; }
    // A bubble (NOP) token.
    static PipelineToken bubble() {
        PipelineToken t;
        t.is_bubble = true;
        return t;
    }

    std::string to_string() const {
        if (is_bubble) {
            return "---";
        }
        if (!opcode.empty()) {
            return opcode + "@" + std::to_string(pc);
        }
        return "instr@" + std::to_string(pc);
    }
};

// ── Pipeline configuration ───────────────────────────────────────────────────
struct PipelineConfig {
    std::vector<PipelineStage> stages;
    std::int64_t execution_width = 1;

    std::size_t num_stages() const { return stages.size(); }

    // Returns an error message if the config is malformed, else std::nullopt.
    std::optional<std::string> validate() const {
        if (stages.size() < 2) {
            return "pipeline must have at least 2 stages, got " +
                   std::to_string(stages.size());
        }
        if (execution_width < 1) {
            return "execution width must be at least 1, got " +
                   std::to_string(execution_width);
        }
        for (std::size_t i = 0; i < stages.size(); ++i) {
            for (std::size_t j = i + 1; j < stages.size(); ++j) {
                if (stages[i].name == stages[j].name) {
                    return "duplicate stage name: " + stages[i].name;
                }
            }
        }
        bool has_fetch = false, has_writeback = false;
        for (const auto& s : stages) {
            if (s.category == StageCategory::Fetch) has_fetch = true;
            if (s.category == StageCategory::Writeback) has_writeback = true;
        }
        if (!has_fetch) {
            return std::string("pipeline must have at least one fetch stage");
        }
        if (!has_writeback) {
            return std::string(
                "pipeline must have at least one writeback stage");
        }
        return std::nullopt;
    }

    static PipelineConfig classic_5_stage() {
        return PipelineConfig{
            {{"IF", "Instruction Fetch", StageCategory::Fetch},
             {"ID", "Instruction Decode", StageCategory::Decode},
             {"EX", "Execute", StageCategory::Execute},
             {"MEM", "Memory Access", StageCategory::Memory},
             {"WB", "Write Back", StageCategory::Writeback}},
            1};
    }

    static PipelineConfig deep_13_stage() {
        return PipelineConfig{
            {{"IF1", "Fetch 1 - TLB lookup", StageCategory::Fetch},
             {"IF2", "Fetch 2 - cache read", StageCategory::Fetch},
             {"IF3", "Fetch 3 - align/buffer", StageCategory::Fetch},
             {"ID1", "Decode 1 - pre-decode", StageCategory::Decode},
             {"ID2", "Decode 2 - full decode", StageCategory::Decode},
             {"ID3", "Decode 3 - register read", StageCategory::Decode},
             {"EX1", "Execute 1 - ALU", StageCategory::Execute},
             {"EX2", "Execute 2 - shift/multiply", StageCategory::Execute},
             {"EX3", "Execute 3 - result select", StageCategory::Execute},
             {"MEM1", "Memory 1 - address calc", StageCategory::Memory},
             {"MEM2", "Memory 2 - cache access", StageCategory::Memory},
             {"MEM3", "Memory 3 - data align", StageCategory::Memory},
             {"WB", "Write Back", StageCategory::Writeback}},
            1};
    }
};

// ── Statistics ───────────────────────────────────────────────────────────────
struct PipelineStats {
    std::int64_t total_cycles = 0;
    std::int64_t instructions_completed = 0;
    std::int64_t stall_cycles = 0;
    std::int64_t flush_cycles = 0;
    std::int64_t bubble_cycles = 0;

    double ipc() const {
        return total_cycles == 0
                   ? 0.0
                   : static_cast<double>(instructions_completed) /
                         static_cast<double>(total_cycles);
    }
    double cpi() const {
        return instructions_completed == 0
                   ? 0.0
                   : static_cast<double>(total_cycles) /
                         static_cast<double>(instructions_completed);
    }
};

// ── Snapshot ─────────────────────────────────────────────────────────────────
struct PipelineSnapshot {
    std::int64_t cycle = 0;
    std::unordered_map<std::string, PipelineToken> stages;
    bool stalled = false;
    bool flushing = false;
    std::int64_t pc = 0;
};

// ── Hazard detection ─────────────────────────────────────────────────────────
enum class HazardAction {
    None,
    ForwardFromEX,
    ForwardFromMEM,
    Stall,
    Flush
};

inline std::string to_string(HazardAction a) {
    switch (a) {
    case HazardAction::None:
        return "NONE";
    case HazardAction::ForwardFromEX:
        return "FORWARD_FROM_EX";
    case HazardAction::ForwardFromMEM:
        return "FORWARD_FROM_MEM";
    case HazardAction::Stall:
        return "STALL";
    case HazardAction::Flush:
        return "FLUSH";
    }
    return "NONE";
}

struct HazardResponse {
    HazardAction action = HazardAction::None;
    std::int64_t forward_value = 0;
    std::string forward_source;
    std::size_t stall_stages = 0;
    std::size_t flush_count = 0;
    std::int64_t redirect_pc = 0;
};

// ── Callback types ───────────────────────────────────────────────────────────
using FetchFn = std::function<std::int64_t(std::int64_t)>;
using DecodeFn = std::function<PipelineToken(std::int64_t, PipelineToken)>;
using ExecuteFn = std::function<PipelineToken(PipelineToken)>;
using MemoryFn = std::function<PipelineToken(PipelineToken)>;
using WritebackFn = std::function<void(const PipelineToken&)>;
using HazardFn = std::function<HazardResponse(
    const std::vector<std::optional<PipelineToken>>&)>;
using PredictFn = std::function<std::int64_t(std::int64_t)>;

// ── The pipeline ─────────────────────────────────────────────────────────────
class Pipeline {
  public:
    Pipeline(PipelineConfig config, FetchFn fetch, DecodeFn decode,
             ExecuteFn execute, MemoryFn memory, WritebackFn writeback)
        : config_(std::move(config)),
          fetch_(std::move(fetch)),
          decode_(std::move(decode)),
          execute_(std::move(execute)),
          memory_(std::move(memory)),
          writeback_(std::move(writeback)) {
        if (auto err = config_.validate()) {
            throw std::invalid_argument(*err);
        }
        stages_.assign(config_.num_stages(), std::nullopt);
    }

    void set_hazard_fn(HazardFn f) { hazard_ = std::move(f); }
    void set_predict_fn(PredictFn f) { predict_ = std::move(f); }
    void set_pc(std::int64_t pc) { pc_ = pc; }
    std::int64_t pc() const { return pc_; }
    std::int64_t cycle() const { return cycle_; }
    bool is_halted() const { return halted_; }
    PipelineStats stats() const { return stats_; }
    const PipelineConfig& config() const { return config_; }

    const PipelineToken* stage_contents(const std::string& name) const {
        for (std::size_t i = 0; i < config_.stages.size(); ++i) {
            if (config_.stages[i].name == name) {
                return stages_[i] ? &*stages_[i] : nullptr;
            }
        }
        return nullptr;
    }

    const std::vector<PipelineSnapshot>& trace() const { return history_; }

    PipelineSnapshot snapshot() const { return take_snapshot(false, false); }

    PipelineSnapshot step() {
        if (halted_) {
            return take_snapshot(false, false);
        }
        cycle_ += 1;
        stats_.total_cycles += 1;
        std::size_t num_stages = config_.num_stages();

        HazardResponse hz;
        if (hazard_) {
            hz = hazard_(stages_);
        }

        std::vector<std::optional<PipelineToken>> next(num_stages, std::nullopt);
        bool stalled = false, flushing = false;

        if (hz.action == HazardAction::Flush) {
            flushing = true;
            stats_.flush_cycles += 1;
            std::size_t flush_count = hz.flush_count;
            if (flush_count == 0) {
                for (std::size_t i = 0; i < config_.stages.size(); ++i) {
                    if (config_.stages[i].category == StageCategory::Execute) {
                        flush_count = i;
                        break;
                    }
                }
                if (flush_count == 0) flush_count = 1;
            }
            if (flush_count > num_stages) flush_count = num_stages;

            for (std::size_t i = num_stages; i-- > flush_count;) {
                if (i > flush_count) {
                    next[i] = stages_[i - 1];
                } else {  // i == flush_count (>= 1)
                    PipelineToken b = PipelineToken::bubble();
                    b.stage_entered[config_.stages[i].name] = cycle_;
                    next[i] = std::move(b);
                }
            }
            for (std::size_t i = 0; i < flush_count; ++i) {
                PipelineToken b = PipelineToken::bubble();
                b.stage_entered[config_.stages[i].name] = cycle_;
                next[i] = std::move(b);
            }
            pc_ = hz.redirect_pc;
            next[0] = fetch_new_instruction();
        } else if (hz.action == HazardAction::Stall) {
            stalled = true;
            stats_.stall_cycles += 1;
            std::size_t stall_point = hz.stall_stages;
            if (stall_point == 0) {
                for (std::size_t i = 0; i < config_.stages.size(); ++i) {
                    if (config_.stages[i].category == StageCategory::Execute) {
                        stall_point = i;
                        break;
                    }
                }
                if (stall_point == 0) stall_point = 1;
            }
            if (stall_point >= num_stages) stall_point = num_stages - 1;

            for (std::size_t i = num_stages; i-- > stall_point + 1;) {
                next[i] = stages_[i - 1];
            }
            {
                PipelineToken b = PipelineToken::bubble();
                b.stage_entered[config_.stages[stall_point].name] = cycle_;
                next[stall_point] = std::move(b);
            }
            for (std::size_t i = 0; i < stall_point; ++i) {
                next[i] = stages_[i];
            }
            // PC does not advance during a stall.
        } else {
            if (hz.action == HazardAction::ForwardFromEX ||
                hz.action == HazardAction::ForwardFromMEM) {
                for (std::size_t i = 0; i < config_.stages.size(); ++i) {
                    if (config_.stages[i].category == StageCategory::Decode) {
                        if (stages_[i] && !stages_[i]->is_bubble) {
                            stages_[i]->alu_result = hz.forward_value;
                            stages_[i]->forwarded_from = hz.forward_source;
                            break;
                        }
                    }
                }
            }
            for (std::size_t i = num_stages; i-- > 1;) {
                next[i] = stages_[i - 1];
            }
            next[0] = fetch_new_instruction();
        }

        stages_ = std::move(next);

        // Stage callbacks, last to first.
        for (std::size_t i = num_stages; i-- > 0;) {
            if (!stages_[i] || stages_[i]->is_bubble) {
                continue;
            }
            StageCategory cat = config_.stages[i].category;
            const std::string& name = config_.stages[i].name;
            stages_[i]->stage_entered.emplace(name, cycle_);  // or_insert

            if (cat == StageCategory::Decode) {
                if (stages_[i]->opcode.empty()) {
                    PipelineToken tok = std::move(*stages_[i]);
                    std::int64_t raw = tok.raw_instruction;
                    stages_[i] = decode_(raw, std::move(tok));
                }
            } else if (cat == StageCategory::Execute) {
                auto it = stages_[i]->stage_entered.find(name);
                if (it != stages_[i]->stage_entered.end() &&
                    it->second == cycle_) {
                    stages_[i] = execute_(std::move(*stages_[i]));
                }
            } else if (cat == StageCategory::Memory) {
                auto it = stages_[i]->stage_entered.find(name);
                if (it != stages_[i]->stage_entered.end() &&
                    it->second == cycle_) {
                    stages_[i] = memory_(std::move(*stages_[i]));
                }
            }
        }

        // Retire the last stage.
        if (stages_[num_stages - 1] && !stages_[num_stages - 1]->is_bubble) {
            const PipelineToken& tok = *stages_[num_stages - 1];
            writeback_(tok);
            stats_.instructions_completed += 1;
            if (tok.is_halt) {
                halted_ = true;
            }
        }

        // Count bubbles.
        for (const auto& slot : stages_) {
            if (slot && slot->is_bubble) {
                stats_.bubble_cycles += 1;
            }
        }

        PipelineSnapshot snap = take_snapshot(stalled, flushing);
        history_.push_back(snap);
        return snap;
    }

    PipelineStats run(std::int64_t max_cycles) {
        while (cycle_ < max_cycles && !halted_) {
            step();
        }
        return stats_;
    }

  private:
    PipelineToken fetch_new_instruction() {
        PipelineToken tok = PipelineToken::make();
        tok.pc = pc_;
        tok.raw_instruction = fetch_(pc_);
        tok.stage_entered[config_.stages[0].name] = cycle_;
        pc_ = predict_ ? predict_(pc_) : pc_ + 4;
        return tok;
    }

    PipelineSnapshot take_snapshot(bool stalled, bool flushing) const {
        PipelineSnapshot snap;
        snap.cycle = cycle_;
        snap.pc = pc_;
        snap.stalled = stalled;
        snap.flushing = flushing;
        for (std::size_t i = 0; i < config_.stages.size(); ++i) {
            if (stages_[i]) {
                snap.stages.emplace(config_.stages[i].name, *stages_[i]);
            }
        }
        return snap;
    }

    PipelineConfig config_;
    std::vector<std::optional<PipelineToken>> stages_;
    std::int64_t pc_ = 0;
    std::int64_t cycle_ = 0;
    bool halted_ = false;
    PipelineStats stats_;
    std::vector<PipelineSnapshot> history_;

    FetchFn fetch_;
    DecodeFn decode_;
    ExecuteFn execute_;
    MemoryFn memory_;
    WritebackFn writeback_;
    HazardFn hazard_;
    PredictFn predict_;
};

}  // namespace cpu_pipeline
}  // namespace ca

#endif  // CPU_PIPELINE_HPP
