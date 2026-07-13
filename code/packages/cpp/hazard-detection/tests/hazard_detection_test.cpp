// Tests for the C++ hazard-detection library, using the header-only iso_test.h
// harness (pure ISO). Cases mirror the Rust crate's own unit tests across the
// data / control / structural detectors and the combined unit.
#include "iso_test.h"

#include <string>

#include "hazard_detection.hpp"

namespace hd = ca::hazard_detection;
using hd::HazardAction;
using hd::PipelineSlot;

static PipelineSlot empty_slot() {
    PipelineSlot s;
    s.valid = false;
    return s;
}

int main() {
    // ── priority ───────────────────────────────────────────────────────────
    ISO_CHECK(hd::priority(HazardAction::None) == 0);
    ISO_CHECK(hd::priority(HazardAction::ForwardFromMEM) == 1);
    ISO_CHECK(hd::priority(HazardAction::ForwardFromEX) == 2);
    ISO_CHECK(hd::priority(HazardAction::Stall) == 3);
    ISO_CHECK(hd::priority(HazardAction::Flush) == 4);

    // ── data hazard ────────────────────────────────────────────────────────
    {
        hd::DataHazardDetector d;
        PipelineSlot id = empty_slot();
        PipelineSlot ex;
        ex.valid = true;
        ex.dest_reg = 1;
        PipelineSlot mem = empty_slot();
        ISO_CHECK(d.detect(id, ex, mem).action == HazardAction::None);

        id = PipelineSlot{};
        id.valid = true;  // no source regs
        ISO_CHECK(d.detect(id, ex, mem).action == HazardAction::None);

        id.source_regs = {2, 3};
        ex.dest_reg = 5;
        mem.valid = true;
        mem.dest_reg = 6;
        ISO_CHECK(d.detect(id, ex, mem).action == HazardAction::None);

        id = PipelineSlot{};
        id.valid = true;
        id.source_regs = {1, 5};
        ex = PipelineSlot{};
        ex.valid = true;
        ex.dest_reg = 1;
        ex.dest_value = 42;
        mem = empty_slot();
        auto r = d.detect(id, ex, mem);
        ISO_CHECK(r.action == HazardAction::ForwardFromEX);
        ISO_CHECK(r.forwarded_value == 42 && r.forwarded_from == "EX");

        id.source_regs = {1};
        ex = empty_slot();
        mem = PipelineSlot{};
        mem.valid = true;
        mem.dest_reg = 1;
        mem.dest_value = 99;
        r = d.detect(id, ex, mem);
        ISO_CHECK(r.action == HazardAction::ForwardFromMEM && r.forwarded_value == 99);
        ISO_CHECK(r.forwarded_from == "MEM");

        // load-use stall
        ex = PipelineSlot{};
        ex.valid = true;
        ex.dest_reg = 1;
        ex.mem_read = true;
        mem = empty_slot();
        r = d.detect(id, ex, mem);
        ISO_CHECK(r.action == HazardAction::Stall && r.stall_cycles == 1);

        // EX priority over MEM
        ex = PipelineSlot{};
        ex.valid = true;
        ex.dest_reg = 1;
        ex.dest_value = 10;
        mem = PipelineSlot{};
        mem.valid = true;
        mem.dest_reg = 1;
        mem.dest_value = 20;
        r = d.detect(id, ex, mem);
        ISO_CHECK(r.action == HazardAction::ForwardFromEX && r.forwarded_value == 10);

        // stall beats forward
        id.source_regs = {1, 2};
        ex = PipelineSlot{};
        ex.valid = true;
        ex.dest_reg = 1;
        ex.mem_read = true;
        mem = PipelineSlot{};
        mem.valid = true;
        mem.dest_reg = 2;
        mem.dest_value = 77;
        ISO_CHECK(d.detect(id, ex, mem).action == HazardAction::Stall);

        // EX dest None / EX invalid -> none
        id.source_regs = {1};
        ex = PipelineSlot{};
        ex.valid = true;  // dest_reg nullopt
        mem = empty_slot();
        ISO_CHECK(d.detect(id, ex, mem).action == HazardAction::None);
        ex.valid = false;
        ex.dest_reg = 1;
        ISO_CHECK(d.detect(id, ex, mem).action == HazardAction::None);
    }

    // ── control hazard ─────────────────────────────────────────────────────
    {
        hd::ControlHazardDetector c;
        ISO_CHECK(c.detect(empty_slot()).action == HazardAction::None);

        PipelineSlot ex;
        ex.valid = true;  // not a branch
        ISO_CHECK(c.detect(ex).action == HazardAction::None);

        ex = PipelineSlot{};
        ex.valid = true;
        ex.is_branch = true;
        ex.branch_taken = true;
        ex.branch_predicted_taken = true;
        ISO_CHECK(c.detect(ex).action == HazardAction::None);
        ex.branch_taken = false;
        ex.branch_predicted_taken = false;
        ISO_CHECK(c.detect(ex).action == HazardAction::None);

        ex = PipelineSlot{};
        ex.valid = true;
        ex.is_branch = true;
        ex.pc = 0x100;
        ex.branch_taken = true;
        ex.branch_predicted_taken = false;
        auto r = c.detect(ex);
        ISO_CHECK(r.action == HazardAction::Flush && r.flush_count == 2);
        ISO_CHECK(r.reason.find("not-taken, actually taken") != std::string::npos);

        ex.branch_taken = false;
        ex.branch_predicted_taken = true;
        r = c.detect(ex);
        ISO_CHECK(r.action == HazardAction::Flush && r.flush_count == 2);
        ISO_CHECK(r.reason.find("taken, actually not-taken") != std::string::npos);
    }

    // ── structural hazard ──────────────────────────────────────────────────
    {
        PipelineSlot id;
        id.valid = true;
        id.uses_alu = true;
        PipelineSlot ex;
        ex.valid = true;
        ex.uses_alu = true;
        ISO_CHECK(hd::StructuralHazardDetector(2, 1, true).detect(id, ex, nullptr, nullptr).action ==
                  HazardAction::None);
        auto r = hd::StructuralHazardDetector(1, 1, true).detect(id, ex, nullptr, nullptr);
        ISO_CHECK(r.action == HazardAction::Stall && r.stall_cycles == 1);

        id = PipelineSlot{};
        id.valid = true;
        id.uses_fp = true;
        ex = PipelineSlot{};
        ex.valid = true;
        ex.uses_fp = true;
        ISO_CHECK(hd::StructuralHazardDetector(1, 1, true).detect(id, ex, nullptr, nullptr).action ==
                  HazardAction::Stall);
        ISO_CHECK(hd::StructuralHazardDetector(1, 2, true).detect(id, ex, nullptr, nullptr).action ==
                  HazardAction::None);

        PipelineSlot empty = empty_slot();
        PipelineSlot ex_alu;
        ex_alu.valid = true;
        ex_alu.uses_alu = true;
        ISO_CHECK(hd::StructuralHazardDetector(1, 1, true).detect(empty, ex_alu, nullptr, nullptr)
                      .action == HazardAction::None);

        PipelineSlot id2;
        id2.valid = true;
        PipelineSlot ex2;
        ex2.valid = true;
        PipelineSlot if_s;
        if_s.valid = true;
        if_s.pc = 0x10;
        PipelineSlot mem_load;
        mem_load.valid = true;
        mem_load.pc = 0x04;
        mem_load.mem_read = true;
        ISO_CHECK(hd::StructuralHazardDetector(1, 1, false).detect(id2, ex2, &if_s, &mem_load)
                      .action == HazardAction::Stall);
        PipelineSlot mem_store;
        mem_store.valid = true;
        mem_store.mem_write = true;
        ISO_CHECK(hd::StructuralHazardDetector(1, 1, false).detect(id2, ex2, &if_s, &mem_store)
                      .action == HazardAction::Stall);
        ISO_CHECK(hd::StructuralHazardDetector(1, 1, true).detect(id2, ex2, &if_s, &mem_load)
                      .action == HazardAction::None);
        PipelineSlot mem_idle;
        mem_idle.valid = true;
        ISO_CHECK(hd::StructuralHazardDetector(1, 1, false).detect(id2, ex2, &if_s, &mem_idle)
                      .action == HazardAction::None);
    }

    // ── combined unit ──────────────────────────────────────────────────────
    {
        PipelineSlot if_s;
        if_s.valid = true;
        PipelineSlot empty = empty_slot();

        {
            hd::HazardUnit unit(2, 1, true);
            PipelineSlot id;
            id.valid = true;
            id.source_regs = {2};
            PipelineSlot ex;
            ex.valid = true;
            ex.dest_reg = 5;
            ISO_CHECK(unit.check(if_s, id, ex, empty).action == HazardAction::None);
        }
        {
            hd::HazardUnit unit(2, 1, true);
            PipelineSlot id;
            id.valid = true;
            id.source_regs = {1};
            PipelineSlot ex;
            ex.valid = true;
            ex.dest_reg = 1;
            ex.dest_value = 42;
            ex.is_branch = true;
            ex.branch_taken = true;
            ex.branch_predicted_taken = false;
            ISO_CHECK(unit.check(if_s, id, ex, empty).action == HazardAction::Flush);
        }
        {
            hd::HazardUnit unit(1, 1, true);
            ISO_CHECK(unit.check(empty, empty, empty, empty).action == HazardAction::None);
        }
        {
            hd::HazardUnit unit(2, 1, true);
            PipelineSlot id;
            id.valid = true;
            id.source_regs = {3};
            PipelineSlot ex = empty_slot();
            PipelineSlot mem;
            mem.valid = true;
            mem.dest_reg = 3;
            mem.dest_value = 88;
            auto r = unit.check(if_s, id, ex, mem);
            ISO_CHECK(r.action == HazardAction::ForwardFromMEM && r.forwarded_value == 88);
        }
        {
            hd::HazardUnit unit(1, 1, true);
            PipelineSlot id;
            id.valid = true;
            id.uses_alu = true;
            PipelineSlot ex;
            ex.valid = true;
            ex.dest_reg = 5;
            ex.uses_alu = true;
            ISO_CHECK(unit.check(if_s, id, ex, empty).action == HazardAction::Stall);
        }
        {
            hd::HazardUnit unit(2, 1, true);
            PipelineSlot id1;
            id1.valid = true;
            id1.source_regs = {2};
            PipelineSlot ex1;
            ex1.valid = true;
            ex1.dest_reg = 5;
            unit.check(if_s, id1, ex1, empty);  // none

            PipelineSlot id2;
            id2.valid = true;
            id2.source_regs = {1};
            PipelineSlot ex2;
            ex2.valid = true;
            ex2.dest_reg = 1;
            ex2.dest_value = 10;
            unit.check(if_s, id2, ex2, empty);  // forward

            PipelineSlot ex3;
            ex3.valid = true;
            ex3.is_branch = true;
            ex3.branch_taken = true;
            ex3.branch_predicted_taken = false;
            unit.check(if_s, empty, ex3, empty);  // flush

            ISO_CHECK(unit.history().size() == 3);
            ISO_CHECK(unit.stall_count() == 0);
            ISO_CHECK(unit.flush_count() == 1);
            ISO_CHECK(unit.forward_count() == 1);
        }
    }

    return ISO_TEST_RESULT();
}
