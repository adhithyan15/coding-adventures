// Tests for cpu-pipeline, mirroring the Rust crate's unit tests across the
// token, snapshot, and pipeline modules, using the header-only iso_test.h.
#include "iso_test.h"

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#include "cpu_pipeline.hpp"

namespace cp = ca::cpu_pipeline;
using Tok = cp::PipelineToken;
using Slots = std::vector<std::optional<Tok>>;

static constexpr double EPS = 1e-3;

enum { OP_NOP = 0x00, OP_ADD = 0x01, OP_LDR = 0x02, OP_BEQ = 0x04,
       OP_HALT = 0xFF };
static std::int64_t make_instruction(std::int64_t op, std::int64_t rd,
                                     std::int64_t rs1, std::int64_t rs2) {
    return (op << 24) | (rd << 16) | (rs1 << 8) | rs2;
}

static cp::FetchFn simple_fetch(std::vector<std::int64_t> instrs) {
    return [instrs](std::int64_t pc) -> std::int64_t {
        std::size_t idx = static_cast<std::size_t>(pc / 4);
        return idx < instrs.size() ? instrs[idx] : 0;
    };
}
static cp::DecodeFn simple_decode() {
    return [](std::int64_t raw, Tok t) -> Tok {
        std::int64_t op = (raw >> 24) & 0xFF, rd = (raw >> 16) & 0xFF,
                     rs1 = (raw >> 8) & 0xFF, rs2 = raw & 0xFF;
        switch (op) {
        case OP_ADD:
            t.opcode = "ADD";
            t.rd = rd;
            t.rs1 = rs1;
            t.rs2 = rs2;
            t.reg_write = true;
            break;
        case OP_LDR:
            t.opcode = "LDR";
            t.rd = rd;
            t.rs1 = rs1;
            t.mem_read = true;
            t.reg_write = true;
            break;
        case 0x03:
            t.opcode = "STR";
            t.rs1 = rs1;
            t.rs2 = rs2;
            t.mem_write = true;
            break;
        case OP_BEQ:
            t.opcode = "BEQ";
            t.rs1 = rs1;
            t.rs2 = rs2;
            t.is_branch = true;
            break;
        case OP_HALT:
            t.opcode = "HALT";
            t.is_halt = true;
            break;
        default:
            t.opcode = "NOP";
            break;
        }
        return t;
    };
}
static cp::ExecuteFn simple_execute() {
    return [](Tok t) -> Tok {
        if (t.opcode == "ADD") {
            t.alu_result = t.rs1 + t.rs2;
        } else if (t.opcode == "LDR" || t.opcode == "STR") {
            t.alu_result = t.rs1 + t.immediate;
        } else if (t.opcode == "BEQ") {
            t.branch_target = t.pc + t.immediate;
        }
        return t;
    };
}
static cp::MemoryFn simple_memory() {
    return [](Tok t) -> Tok {
        if (t.mem_read) {
            t.mem_data = 42;
            t.write_data = t.mem_data;
        } else {
            t.write_data = t.alu_result;
        }
        return t;
    };
}

static std::vector<std::int64_t> adds(std::size_t n) {
    return std::vector<std::int64_t>(n, make_instruction(OP_ADD, 1, 2, 3));
}

int main() {
    // ══ Token ═════════════════════════════════════════════════════════════
    {
        Tok t = Tok::make();
        ISO_CHECK_EQ_INT(t.rs1, -1);
        ISO_CHECK_EQ_INT(t.rs2, -1);
        ISO_CHECK_EQ_INT(t.rd, -1);
        ISO_CHECK(!t.is_bubble);
        ISO_CHECK(t.stage_entered.empty());
    }
    {
        Tok b = Tok::bubble();
        ISO_CHECK(b.is_bubble);
        ISO_CHECK(b.to_string() == "---");
    }
    {
        Tok t = Tok::make();
        t.opcode = "ADD";
        t.pc = 100;
        ISO_CHECK(t.to_string() == "ADD@100");
        Tok t2 = Tok::make();
        t2.pc = 200;
        ISO_CHECK(t2.to_string() == "instr@200");
    }
    {
        // clone independence (value semantics)
        Tok t = Tok::make();
        t.pc = 100;
        t.opcode = "ADD";
        t.stage_entered["IF"] = 1;
        Tok c = t;
        c.stage_entered["EX"] = 3;
        ISO_CHECK(t.stage_entered.find("EX") == t.stage_entered.end());
    }

    // ══ StageCategory / HazardAction strings ══════════════════════════════
    ISO_CHECK(cp::to_string(cp::StageCategory::Fetch) == "fetch");
    ISO_CHECK(cp::to_string(cp::StageCategory::Decode) == "decode");
    ISO_CHECK(cp::to_string(cp::StageCategory::Execute) == "execute");
    ISO_CHECK(cp::to_string(cp::StageCategory::Memory) == "memory");
    ISO_CHECK(cp::to_string(cp::StageCategory::Writeback) == "writeback");
    ISO_CHECK(cp::to_string(cp::HazardAction::None) == "NONE");
    ISO_CHECK(cp::to_string(cp::HazardAction::ForwardFromEX) ==
              "FORWARD_FROM_EX");
    ISO_CHECK(cp::to_string(cp::HazardAction::ForwardFromMEM) ==
              "FORWARD_FROM_MEM");
    ISO_CHECK(cp::to_string(cp::HazardAction::Stall) == "STALL");
    ISO_CHECK(cp::to_string(cp::HazardAction::Flush) == "FLUSH");

    // ══ Config presets + validation ═══════════════════════════════════════
    {
        auto c = cp::PipelineConfig::classic_5_stage();
        ISO_CHECK_EQ_UINT(c.num_stages(), 5u);
        ISO_CHECK(!c.validate().has_value());
        ISO_CHECK(c.stages[0].name == "IF");
        ISO_CHECK(c.stages[4].name == "WB");
    }
    {
        auto c = cp::PipelineConfig::deep_13_stage();
        ISO_CHECK_EQ_UINT(c.num_stages(), 13u);
        ISO_CHECK(!c.validate().has_value());
    }
    {
        using C = cp::StageCategory;
        cp::PipelineConfig c1{{{"IF", "", C::Fetch}}, 1};
        ISO_CHECK(c1.validate().has_value());
        cp::PipelineConfig c2{{{"IF", "", C::Fetch}, {"WB", "", C::Writeback}},
                              0};
        ISO_CHECK(c2.validate().has_value());
        cp::PipelineConfig c3{{{"IF", "", C::Fetch}, {"IF", "", C::Writeback}},
                              1};
        ISO_CHECK(c3.validate().has_value());
        cp::PipelineConfig c4{{{"EX", "", C::Execute}, {"WB", "", C::Writeback}},
                              1};
        ISO_CHECK(c4.validate().has_value());
        cp::PipelineConfig c5{{{"IF", "", C::Fetch}, {"EX", "", C::Execute}}, 1};
        ISO_CHECK(c5.validate().has_value());
        cp::PipelineConfig c6{{{"IF", "", C::Fetch}, {"WB", "", C::Writeback}},
                              1};
        ISO_CHECK(!c6.validate().has_value());
    }

    // ══ Stats: IPC / CPI ══════════════════════════════════════════════════
    {
        cp::PipelineStats s;
        s.total_cycles = 100;
        s.instructions_completed = 80;
        ISO_CHECK_EQ_DBL(s.ipc(), 0.8, EPS);
        cp::PipelineStats s2;
        s2.total_cycles = 120;
        s2.instructions_completed = 100;
        ISO_CHECK_EQ_DBL(s2.cpi(), 1.2, EPS);
        cp::PipelineStats s3;
        ISO_CHECK_EQ_DBL(s3.ipc(), 0.0, EPS);
        s3.total_cycles = 10;
        ISO_CHECK_EQ_DBL(s3.cpi(), 0.0, EPS);
    }

    auto make_pipeline = [](std::vector<std::int64_t> instrs,
                            cp::WritebackFn wb) {
        return cp::Pipeline(cp::PipelineConfig::classic_5_stage(),
                            simple_fetch(std::move(instrs)), simple_decode(),
                            simple_execute(), simple_memory(), std::move(wb));
    };
    cp::WritebackFn noop_wb = [](const Tok&) {};

    // ══ Basic pipeline ════════════════════════════════════════════════════
    {
        auto p = make_pipeline({make_instruction(OP_ADD, 1, 2, 3)}, noop_wb);
        ISO_CHECK(!p.is_halted());
        ISO_CHECK_EQ_INT(p.cycle(), 0);
        ISO_CHECK_EQ_INT(p.pc(), 0);
        ISO_CHECK_EQ_UINT(p.config().num_stages(), 5u);
    }
    {
        bool threw = false;
        try {
            cp::Pipeline bad(
                cp::PipelineConfig{{{"IF", "", cp::StageCategory::Fetch}}, 1},
                simple_fetch({}), simple_decode(), simple_execute(),
                simple_memory(), noop_wb);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }
    {
        std::vector<std::int64_t> completed;
        cp::WritebackFn wb = [&completed](const Tok& t) {
            completed.push_back(t.pc);
        };
        std::vector<std::int64_t> instrs = {make_instruction(OP_ADD, 1, 2, 3),
                                            0, 0, 0, 0};
        auto p = make_pipeline(instrs, wb);
        for (int i = 0; i < 5; ++i) p.step();
        ISO_CHECK(!completed.empty());
        ISO_CHECK_EQ_INT(completed[0], 0);
    }
    {
        std::vector<std::int64_t> completed;
        cp::WritebackFn wb = [&completed](const Tok& t) {
            completed.push_back(t.pc);
        };
        auto p = make_pipeline(adds(20), wb);
        for (int i = 0; i < 4; ++i) p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 0u);
        p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 1u);
        p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 2u);
        p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 3u);
    }
    {
        auto p = make_pipeline(adds(100), noop_wb);
        for (int i = 0; i < 50; ++i) p.step();
        auto st = p.stats();
        ISO_CHECK_EQ_INT(st.instructions_completed, 46);
        ISO_CHECK(st.ipc() > 0.85 && st.ipc() < 1.01);
    }
    {
        std::vector<std::int64_t> completed;
        cp::WritebackFn wb = [&completed](const Tok& t) {
            completed.push_back(t.pc);
        };
        std::vector<std::int64_t> instrs = {make_instruction(OP_ADD, 1, 2, 3),
                                            make_instruction(OP_ADD, 4, 5, 6),
                                            make_instruction(OP_HALT, 0, 0, 0),
                                            0, 0};
        auto p = make_pipeline(instrs, wb);
        auto st = p.run(100);
        ISO_CHECK(p.is_halted());
        ISO_CHECK_EQ_INT(p.cycle(), 7);
        ISO_CHECK_EQ_INT(st.instructions_completed, 3);
        ISO_CHECK_EQ_UINT(completed.size(), 3u);
    }
    {
        auto p = make_pipeline({}, noop_wb);
        auto snap = p.step();
        ISO_CHECK_EQ_INT(snap.cycle, 1);
    }

    // ══ Stall ═════════════════════════════════════════════════════════════
    {
        std::vector<std::int64_t> instrs = {make_instruction(OP_LDR, 1, 2, 0),
                                            make_instruction(OP_ADD, 3, 1, 4),
                                            make_instruction(OP_ADD, 5, 6, 7),
                                            0, 0, 0, 0, 0};
        auto p = make_pipeline(instrs, noop_wb);
        bool injected = false;
        p.set_hazard_fn([&injected](const Slots& s) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (!injected && s.size() >= 3 && s[2] && s[1] &&
                !s[2]->is_bubble && s[2]->opcode == "LDR" && !s[1]->is_bubble &&
                s[1]->opcode == "ADD") {
                injected = true;
                r.action = cp::HazardAction::Stall;
                r.stall_stages = 2;
            }
            return r;
        });
        p.step();
        p.step();
        p.step();
        auto snap = p.step();
        ISO_CHECK(snap.stalled);
        const Tok* ex = p.stage_contents("EX");
        ISO_CHECK(ex && ex->is_bubble);
        const Tok* id = p.stage_contents("ID");
        ISO_CHECK(id && id->opcode == "ADD");
        ISO_CHECK_EQ_INT(p.stats().stall_cycles, 1);
    }
    {
        auto p = make_pipeline(adds(10), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count == 3) {
                r.action = cp::HazardAction::Stall;
                r.stall_stages = 2;
            }
            return r;
        });
        for (int i = 0; i < 3; ++i) p.step();
        const Tok* ex = p.stage_contents("EX");
        ISO_CHECK(ex && ex->is_bubble);
    }
    {
        auto p = make_pipeline(adds(20), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count == 3) {
                r.action = cp::HazardAction::Stall;
                r.stall_stages = 0;
            }
            return r;
        });
        for (int i = 0; i < 5; ++i) p.step();
        ISO_CHECK_EQ_INT(p.stats().stall_cycles, 1);
    }
    {
        auto p = make_pipeline(adds(20), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count == 3) {
                r.action = cp::HazardAction::Stall;
                r.stall_stages = 100;
            }
            return r;
        });
        for (int i = 0; i < 10; ++i) p.step();
        ISO_CHECK(p.stats().stall_cycles >= 1);
    }
    {
        auto p = make_pipeline(adds(50), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count % 5 == 0) {
                r.action = cp::HazardAction::Stall;
                r.stall_stages = 2;
            }
            return r;
        });
        for (int i = 0; i < 30; ++i) p.step();
        auto st = p.stats();
        ISO_CHECK(st.ipc() < 1.0);
        ISO_CHECK(st.stall_cycles > 0);
    }

    // ══ Flush ═════════════════════════════════════════════════════════════
    {
        std::vector<std::int64_t> instrs = {make_instruction(OP_BEQ, 0, 1, 2),
                                            make_instruction(OP_ADD, 1, 2, 3),
                                            make_instruction(OP_ADD, 4, 5, 6),
                                            0, 0,
                                            make_instruction(OP_ADD, 7, 8, 9),
                                            0, 0};
        auto p = make_pipeline(instrs, noop_wb);
        bool flushed = false;
        p.set_hazard_fn([&flushed](const Slots& s) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (!flushed && s.size() >= 3 && s[2] && !s[2]->is_bubble &&
                s[2]->is_branch) {
                flushed = true;
                r.action = cp::HazardAction::Flush;
                r.flush_count = 2;
                r.redirect_pc = 20;
            }
            return r;
        });
        p.step();
        p.step();
        p.step();
        auto snap = p.step();
        ISO_CHECK(snap.flushing);
        ISO_CHECK_EQ_INT(p.pc(), 24);
        ISO_CHECK_EQ_INT(p.stats().flush_cycles, 1);
    }
    {
        auto p = make_pipeline(adds(20), noop_wb);
        bool flushed = false;
        p.set_hazard_fn([&flushed](const Slots& s) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (!flushed && s.size() >= 3 && s[2] && !s[2]->is_bubble) {
                flushed = true;
                r.action = cp::HazardAction::Flush;
                r.flush_count = 0;
                r.redirect_pc = 100;
            }
            return r;
        });
        for (int i = 0; i < 5; ++i) p.step();
        ISO_CHECK_EQ_INT(p.stats().flush_cycles, 1);
    }
    {
        auto p = make_pipeline(adds(20), noop_wb);
        bool flushed = false;
        p.set_hazard_fn([&flushed](const Slots& s) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (!flushed && s.size() >= 3 && s[2] && !s[2]->is_bubble) {
                flushed = true;
                r.action = cp::HazardAction::Flush;
                r.flush_count = 100;
                r.redirect_pc = 0;
            }
            return r;
        });
        for (int i = 0; i < 10; ++i) p.step();
        ISO_CHECK_EQ_INT(p.stats().flush_cycles, 1);
    }
    {
        auto p = make_pipeline(adds(50), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            int c = ++count;
            if (c == 5 || c == 10) {
                r.action = cp::HazardAction::Stall;
                r.stall_stages = 2;
            } else if (c == 15) {
                r.action = cp::HazardAction::Flush;
                r.flush_count = 2;
                r.redirect_pc = 0;
            }
            return r;
        });
        for (int i = 0; i < 20; ++i) p.step();
        auto st = p.stats();
        ISO_CHECK_EQ_INT(st.stall_cycles, 2);
        ISO_CHECK_EQ_INT(st.flush_cycles, 1);
    }

    {
        // SIZE_MAX flush_count must clamp (no signed-narrowing OOB)
        auto p = make_pipeline(adds(20), noop_wb);
        bool flushed = false;
        p.set_hazard_fn([&flushed](const Slots& s) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (!flushed && s.size() >= 3 && s[2] && !s[2]->is_bubble) {
                flushed = true;
                r.action = cp::HazardAction::Flush;
                r.flush_count = static_cast<std::size_t>(-1);
                r.redirect_pc = 0;
            }
            return r;
        });
        for (int i = 0; i < 10; ++i) p.step();
        ISO_CHECK_EQ_INT(p.stats().flush_cycles, 1);
    }
    {
        // SIZE_MAX stall_stages must clamp
        auto p = make_pipeline(adds(20), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count == 3) {
                r.action = cp::HazardAction::Stall;
                r.stall_stages = static_cast<std::size_t>(-1);
            }
            return r;
        });
        for (int i = 0; i < 10; ++i) p.step();
        ISO_CHECK(p.stats().stall_cycles >= 1);
    }

    // ══ Forwarding ════════════════════════════════════════════════════════
    {
        auto p = make_pipeline(adds(10), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count == 4) {
                r.action = cp::HazardAction::ForwardFromEX;
                r.forward_value = 99;
                r.forward_source = "EX";
            }
            return r;
        });
        for (int i = 0; i < 4; ++i) p.step();
        const Tok* ex = p.stage_contents("EX");
        ISO_CHECK(ex && ex->forwarded_from == "EX");
    }
    {
        auto p = make_pipeline(adds(10), noop_wb);
        int count = 0;
        p.set_hazard_fn([&count](const Slots&) -> cp::HazardResponse {
            cp::HazardResponse r;
            if (++count == 4) {
                r.action = cp::HazardAction::ForwardFromMEM;
                r.forward_value = 77;
                r.forward_source = "MEM";
            }
            return r;
        });
        for (int i = 0; i < 4; ++i) p.step();
        const Tok* ex = p.stage_contents("EX");
        ISO_CHECK(ex && ex->forwarded_from == "MEM");
    }

    // ══ Snapshot / trace ══════════════════════════════════════════════════
    {
        std::vector<std::int64_t> instrs = {make_instruction(OP_ADD, 1, 2, 3),
                                            make_instruction(OP_ADD, 4, 5, 6),
                                            0};
        auto p = make_pipeline(instrs, noop_wb);
        auto s1 = p.step();
        ISO_CHECK_EQ_INT(s1.cycle, 1);
        ISO_CHECK(s1.stages.at("IF").pc == 0);
        auto s2 = p.step();
        ISO_CHECK_EQ_INT(s2.cycle, 2);
        ISO_CHECK(s2.stages.at("ID").pc == 0);
    }
    {
        auto p = make_pipeline(adds(10), noop_wb);
        for (int i = 0; i < 7; ++i) p.step();
        const auto& tr = p.trace();
        ISO_CHECK_EQ_UINT(tr.size(), 7u);
        for (std::size_t i = 0; i < tr.size(); ++i) {
            ISO_CHECK_EQ_INT(tr[i].cycle, static_cast<std::int64_t>(i + 1));
        }
    }
    {
        auto p = make_pipeline({make_instruction(OP_ADD, 1, 2, 3)}, noop_wb);
        p.step();
        auto s1 = p.snapshot();
        auto s2 = p.snapshot();
        ISO_CHECK_EQ_INT(s1.cycle, s2.cycle);
    }

    // ══ Deep / custom / two-stage configs ═════════════════════════════════
    {
        cp::Pipeline p(cp::PipelineConfig::deep_13_stage(),
                       simple_fetch(adds(30)), simple_decode(), simple_execute(),
                       simple_memory(), noop_wb);
        for (int i = 0; i < 12; ++i) p.step();
        ISO_CHECK_EQ_INT(p.stats().instructions_completed, 0);
        p.step();
        ISO_CHECK_EQ_INT(p.stats().instructions_completed, 1);
    }
    {
        using C = cp::StageCategory;
        cp::PipelineConfig cfg{{{"IF", "Fetch", C::Fetch},
                                {"EX", "Execute", C::Execute},
                                {"WB", "Writeback", C::Writeback}},
                               1};
        std::vector<std::int64_t> completed;
        cp::WritebackFn wb = [&completed](const Tok& t) {
            completed.push_back(t.pc);
        };
        cp::Pipeline p(cfg, simple_fetch(adds(10)), simple_decode(),
                       simple_execute(), simple_memory(), wb);
        for (int i = 0; i < 2; ++i) p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 0u);
        p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 1u);
    }
    {
        using C = cp::StageCategory;
        cp::PipelineConfig cfg{
            {{"IF", "Fetch", C::Fetch}, {"WB", "Writeback", C::Writeback}}, 1};
        std::vector<std::int64_t> completed;
        cp::WritebackFn wb = [&completed](const Tok& t) {
            completed.push_back(t.pc);
        };
        cp::Pipeline p(cfg, simple_fetch(adds(10)), simple_decode(),
                       simple_execute(), simple_memory(), wb);
        p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 0u);
        p.step();
        ISO_CHECK_EQ_UINT(completed.size(), 1u);
    }

    // ══ Predict / set_pc / decode / halted / stage_contents / no-hazard ═══
    {
        auto p = make_pipeline(adds(100), noop_wb);
        p.set_predict_fn([](std::int64_t pc) { return pc + 8; });
        p.step();
        ISO_CHECK_EQ_INT(p.pc(), 8);
        p.step();
        ISO_CHECK_EQ_INT(p.pc(), 16);
    }
    {
        auto p = make_pipeline(adds(10), noop_wb);
        p.set_pc(100);
        ISO_CHECK_EQ_INT(p.pc(), 100);
    }
    {
        std::vector<std::int64_t> instrs = {make_instruction(OP_LDR, 5, 3, 0),
                                            0};
        auto p = make_pipeline(instrs, noop_wb);
        p.step();
        p.step();
        const Tok* id = p.stage_contents("ID");
        ISO_CHECK(id);
        ISO_CHECK(id->opcode == "LDR");
        ISO_CHECK_EQ_INT(id->rd, 5);
        ISO_CHECK(id->mem_read);
        ISO_CHECK(id->reg_write);
    }
    {
        std::vector<std::int64_t> instrs = {make_instruction(OP_HALT, 0, 0, 0),
                                            0, 0, 0, 0};
        auto p = make_pipeline(instrs, noop_wb);
        p.run(100);
        std::int64_t cyc = p.cycle();
        p.step();
        p.step();
        ISO_CHECK_EQ_INT(p.cycle(), cyc);
    }
    {
        auto p = make_pipeline(adds(100), noop_wb);
        auto st = p.run(10);
        ISO_CHECK_EQ_INT(st.total_cycles, 10);
        ISO_CHECK(!p.is_halted());
    }
    {
        auto p = make_pipeline({make_instruction(OP_NOP, 0, 0, 0)}, noop_wb);
        p.step();
        ISO_CHECK(p.stage_contents("NONEXISTENT") == nullptr);
    }
    {
        auto p = make_pipeline(adds(20), noop_wb);
        for (int i = 0; i < 10; ++i) p.step();
        auto st = p.stats();
        ISO_CHECK_EQ_INT(st.stall_cycles, 0);
        ISO_CHECK_EQ_INT(st.flush_cycles, 0);
    }

    return ISO_TEST_RESULT();
}
