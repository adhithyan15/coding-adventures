/*
 * Tests for the C hazard-detection library, using the header-only iso_test.h
 * harness (pure ISO). Cases mirror the Rust crate's own unit tests across the
 * data / control / structural detectors and the combined unit.
 */
#include "iso_test.h"

#include <string.h> /* strcmp, strstr */

#include "hazard_detection.h"

/* An empty (invalid) slot: zero-initialised. */
static HdPipelineSlot empty_slot(void) {
    HdPipelineSlot s = {0};
    return s;
}

/* A valid slot with the given source registers (borrowed). */
static HdPipelineSlot slot_with_srcs(const uint32_t *srcs, size_t n) {
    HdPipelineSlot s = {0};
    s.valid = 1;
    s.source_regs = srcs;
    s.num_source_regs = n;
    return s;
}

int main(void) {
    /* ── priority ordering ────────────────────────────────────────────── */
    ISO_CHECK(hd_priority(HD_NONE) == 0);
    ISO_CHECK(hd_priority(HD_FORWARD_FROM_MEM) == 1);
    ISO_CHECK(hd_priority(HD_FORWARD_FROM_EX) == 2);
    ISO_CHECK(hd_priority(HD_STALL) == 3);
    ISO_CHECK(hd_priority(HD_FLUSH) == 4);

    /* ── data hazard ──────────────────────────────────────────────────── */
    {
        /* no hazard when ID empty */
        HdPipelineSlot id = empty_slot();
        HdPipelineSlot ex = {0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        HdPipelineSlot mem = empty_slot();
        ISO_CHECK(hd_data_detect(&id, &ex, &mem).action == HD_NONE);

        /* no hazard when no source registers */
        id = slot_with_srcs(NULL, 0);
        ISO_CHECK(hd_data_detect(&id, &ex, &mem).action == HD_NONE);

        /* no hazard when no dependency */
        uint32_t s23[2] = {2, 3};
        id = slot_with_srcs(s23, 2);
        ex.dest_reg = 5;
        mem.valid = 1;
        mem.has_dest_reg = 1;
        mem.dest_reg = 6;
        ISO_CHECK(hd_data_detect(&id, &ex, &mem).action == HD_NONE);

        /* forward from EX (with value) */
        uint32_t s15[2] = {1, 5};
        id = slot_with_srcs(s15, 2);
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        ex.has_dest_value = 1;
        ex.dest_value = 42;
        mem = empty_slot();
        HdHazardResult r = hd_data_detect(&id, &ex, &mem);
        ISO_CHECK(r.action == HD_FORWARD_FROM_EX);
        ISO_CHECK(r.has_forwarded_value && r.forwarded_value == 42);
        ISO_CHECK(strcmp(r.forwarded_from, "EX") == 0);

        /* forward from MEM */
        uint32_t s1[1] = {1};
        id = slot_with_srcs(s1, 1);
        ex = empty_slot();
        mem = (HdPipelineSlot){0};
        mem.valid = 1;
        mem.has_dest_reg = 1;
        mem.dest_reg = 1;
        mem.has_dest_value = 1;
        mem.dest_value = 99;
        r = hd_data_detect(&id, &ex, &mem);
        ISO_CHECK(r.action == HD_FORWARD_FROM_MEM && r.forwarded_value == 99);
        ISO_CHECK(strcmp(r.forwarded_from, "MEM") == 0);

        /* load-use stall */
        id = slot_with_srcs(s1, 1);
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        ex.mem_read = 1;
        mem = empty_slot();
        r = hd_data_detect(&id, &ex, &mem);
        ISO_CHECK(r.action == HD_STALL && r.stall_cycles == 1);

        /* EX priority over MEM (both match R1) */
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        ex.has_dest_value = 1;
        ex.dest_value = 10;
        mem = (HdPipelineSlot){0};
        mem.valid = 1;
        mem.has_dest_reg = 1;
        mem.dest_reg = 1;
        mem.has_dest_value = 1;
        mem.dest_value = 20;
        r = hd_data_detect(&id, &ex, &mem);
        ISO_CHECK(r.action == HD_FORWARD_FROM_EX && r.forwarded_value == 10);

        /* stall beats forward across registers */
        uint32_t s12[2] = {1, 2};
        id = slot_with_srcs(s12, 2);
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        ex.mem_read = 1;
        mem = (HdPipelineSlot){0};
        mem.valid = 1;
        mem.has_dest_reg = 1;
        mem.dest_reg = 2;
        mem.has_dest_value = 1;
        mem.dest_value = 77;
        ISO_CHECK(hd_data_detect(&id, &ex, &mem).action == HD_STALL);

        /* no hazard when EX dest_reg is None */
        id = slot_with_srcs(s1, 1);
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 0;
        mem = empty_slot();
        ISO_CHECK(hd_data_detect(&id, &ex, &mem).action == HD_NONE);

        /* no hazard when EX invalid */
        ex = (HdPipelineSlot){0};
        ex.valid = 0;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        ISO_CHECK(hd_data_detect(&id, &ex, &mem).action == HD_NONE);
    }

    /* ── control hazard ───────────────────────────────────────────────── */
    {
        HdPipelineSlot e = empty_slot();
        ISO_CHECK(hd_control_detect(&e).action == HD_NONE);

        HdPipelineSlot ex = {0};
        ex.valid = 1;
        ex.is_branch = 0;
        ISO_CHECK(hd_control_detect(&ex).action == HD_NONE);

        /* correctly predicted (taken/taken and not/not) */
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.is_branch = 1;
        ex.branch_taken = 1;
        ex.branch_predicted_taken = 1;
        ISO_CHECK(hd_control_detect(&ex).action == HD_NONE);
        ex.branch_taken = 0;
        ex.branch_predicted_taken = 0;
        ISO_CHECK(hd_control_detect(&ex).action == HD_NONE);

        /* misprediction: predicted not-taken, actually taken */
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.is_branch = 1;
        ex.pc = 0x100;
        ex.branch_taken = 1;
        ex.branch_predicted_taken = 0;
        HdHazardResult r = hd_control_detect(&ex);
        ISO_CHECK(r.action == HD_FLUSH && r.flush_count == 2);
        ISO_CHECK(strstr(r.reason, "not-taken, actually taken") != NULL);

        /* misprediction: predicted taken, actually not-taken */
        ex.branch_taken = 0;
        ex.branch_predicted_taken = 1;
        r = hd_control_detect(&ex);
        ISO_CHECK(r.action == HD_FLUSH && r.flush_count == 2);
        ISO_CHECK(strstr(r.reason, "taken, actually not-taken") != NULL);
    }

    /* ── structural hazard ────────────────────────────────────────────── */
    {
        HdPipelineSlot id = {0};
        id.valid = 1;
        id.uses_alu = 1;
        HdPipelineSlot ex = {0};
        ex.valid = 1;
        ex.uses_alu = 1;

        /* enough ALUs -> none; one ALU -> stall */
        ISO_CHECK(hd_structural_detect(2, 1, 1, &id, &ex, NULL, NULL).action == HD_NONE);
        HdHazardResult r = hd_structural_detect(1, 1, 1, &id, &ex, NULL, NULL);
        ISO_CHECK(r.action == HD_STALL && r.stall_cycles == 1);

        /* FP conflict with one FP unit -> stall; two -> none */
        id = (HdPipelineSlot){0};
        id.valid = 1;
        id.uses_fp = 1;
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.uses_fp = 1;
        ISO_CHECK(hd_structural_detect(1, 1, 1, &id, &ex, NULL, NULL).action == HD_STALL);
        ISO_CHECK(hd_structural_detect(1, 2, 1, &id, &ex, NULL, NULL).action == HD_NONE);

        /* no conflict when ID empty */
        HdPipelineSlot empty = empty_slot();
        HdPipelineSlot ex_alu = {0};
        ex_alu.valid = 1;
        ex_alu.uses_alu = 1;
        ISO_CHECK(hd_structural_detect(1, 1, 1, &empty, &ex_alu, NULL, NULL).action == HD_NONE);

        /* memory-port conflict on shared cache (load and store) */
        HdPipelineSlot id2 = {0};
        id2.valid = 1;
        HdPipelineSlot ex2 = {0};
        ex2.valid = 1;
        HdPipelineSlot if_s = {0};
        if_s.valid = 1;
        if_s.pc = 0x10;
        HdPipelineSlot mem_load = {0};
        mem_load.valid = 1;
        mem_load.pc = 0x04;
        mem_load.mem_read = 1;
        ISO_CHECK(hd_structural_detect(1, 1, 0, &id2, &ex2, &if_s, &mem_load).action == HD_STALL);
        HdPipelineSlot mem_store = {0};
        mem_store.valid = 1;
        mem_store.mem_write = 1;
        ISO_CHECK(hd_structural_detect(1, 1, 0, &id2, &ex2, &if_s, &mem_store).action == HD_STALL);

        /* split cache -> no memory conflict */
        ISO_CHECK(hd_structural_detect(1, 1, 1, &id2, &ex2, &if_s, &mem_load).action == HD_NONE);

        /* MEM not accessing memory -> no conflict */
        HdPipelineSlot mem_idle = {0};
        mem_idle.valid = 1;
        ISO_CHECK(hd_structural_detect(1, 1, 0, &id2, &ex2, &if_s, &mem_idle).action == HD_NONE);
    }

    /* ── combined unit ────────────────────────────────────────────────── */
    {
        HdHazardUnit unit;
        hd_unit_init(&unit, 2, 1, 1);
        HdPipelineSlot if_s = {0};
        if_s.valid = 1;
        HdPipelineSlot empty = empty_slot();

        /* no hazard */
        uint32_t s2[1] = {2};
        HdPipelineSlot id = slot_with_srcs(s2, 1);
        HdPipelineSlot ex = {0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 5;
        ISO_CHECK(hd_unit_check(&unit, &if_s, &id, &ex, &empty).action == HD_NONE);
        hd_unit_free(&unit);

        /* flush beats forward */
        hd_unit_init(&unit, 2, 1, 1);
        uint32_t s1[1] = {1};
        id = slot_with_srcs(s1, 1);
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 1;
        ex.has_dest_value = 1;
        ex.dest_value = 42;
        ex.is_branch = 1;
        ex.branch_taken = 1;
        ex.branch_predicted_taken = 0;
        ISO_CHECK(hd_unit_check(&unit, &if_s, &id, &ex, &empty).action == HD_FLUSH);
        hd_unit_free(&unit);

        /* all empty -> none */
        hd_unit_init(&unit, 1, 1, 1);
        ISO_CHECK(hd_unit_check(&unit, &empty, &empty, &empty, &empty).action == HD_NONE);
        hd_unit_free(&unit);

        /* forward from MEM */
        hd_unit_init(&unit, 2, 1, 1);
        uint32_t s3[1] = {3};
        id = slot_with_srcs(s3, 1);
        ex = empty_slot();
        HdPipelineSlot mem = {0};
        mem.valid = 1;
        mem.has_dest_reg = 1;
        mem.dest_reg = 3;
        mem.has_dest_value = 1;
        mem.dest_value = 88;
        HdHazardResult r = hd_unit_check(&unit, &if_s, &id, &ex, &mem);
        ISO_CHECK(r.action == HD_FORWARD_FROM_MEM && r.forwarded_value == 88);
        hd_unit_free(&unit);

        /* structural stall with one ALU (no source regs, both use ALU) */
        hd_unit_init(&unit, 1, 1, 1);
        id = (HdPipelineSlot){0};
        id.valid = 1;
        id.uses_alu = 1;
        ex = (HdPipelineSlot){0};
        ex.valid = 1;
        ex.has_dest_reg = 1;
        ex.dest_reg = 5;
        ex.uses_alu = 1;
        ISO_CHECK(hd_unit_check(&unit, &if_s, &id, &ex, &empty).action == HD_STALL);
        hd_unit_free(&unit);

        /* statistics tracking across three cycles */
        hd_unit_init(&unit, 2, 1, 1);
        HdPipelineSlot id1 = slot_with_srcs(s2, 1);
        HdPipelineSlot ex1 = {0};
        ex1.valid = 1;
        ex1.has_dest_reg = 1;
        ex1.dest_reg = 5;
        hd_unit_check(&unit, &if_s, &id1, &ex1, &empty); /* none */

        HdPipelineSlot id2 = slot_with_srcs(s1, 1);
        HdPipelineSlot ex2 = {0};
        ex2.valid = 1;
        ex2.has_dest_reg = 1;
        ex2.dest_reg = 1;
        ex2.has_dest_value = 1;
        ex2.dest_value = 10;
        hd_unit_check(&unit, &if_s, &id2, &ex2, &empty); /* forward EX */

        HdPipelineSlot ex3 = {0};
        ex3.valid = 1;
        ex3.is_branch = 1;
        ex3.branch_taken = 1;
        ex3.branch_predicted_taken = 0;
        hd_unit_check(&unit, &if_s, &empty, &ex3, &empty); /* flush */

        size_t hlen = 0;
        (void)hd_unit_history(&unit, &hlen);
        ISO_CHECK(hlen == 3);
        ISO_CHECK(hd_unit_stall_count(&unit) == 0);
        ISO_CHECK(hd_unit_flush_count(&unit) == 1);
        ISO_CHECK(hd_unit_forward_count(&unit) == 1);
        hd_unit_free(&unit);
    }

    return ISO_TEST_RESULT();
}
