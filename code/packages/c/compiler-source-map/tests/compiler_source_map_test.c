/*
 * Tests for the C compiler-source-map sidecar, using the header-only iso_test.h
 * harness (pure ISO). Vectors mirror the Rust crate's own unit tests — each
 * segment's add/lookup (forward and reverse), and the SourceMapChain's
 * end-to-end source↔machine-code round-trips, including optimiser passes and
 * deletions.
 */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "compiler_source_map.h"

int main(void) {
    /* ── SourcePosition display ─────────────────────────────────────────── */
    {
        SmapPosition p = {"hello.bf", 1, 3, 1};
        char buf[64];
        int n = smap_position_to_string(&p, buf, sizeof buf);
        ISO_CHECK(n > 0);
        ISO_CHECK_STR_EQ(buf, "hello.bf:1:3 (len=1)");
    }

    /* ── SourceToAst: add, lookup, multiple entries ─────────────────────── */
    {
        SmapSourceToAst *s2a = smap_s2a_new();
        SmapPosition p = {"hello.bf", 1, 1, 1};
        ISO_CHECK(smap_s2a_add(s2a, &p, 42) == 0);
        const SmapPosition *f = smap_s2a_lookup_by_node_id(s2a, 42);
        ISO_CHECK(f != NULL);
        if (f) ISO_CHECK_STR_EQ(f->file, "hello.bf");
        ISO_CHECK(smap_s2a_lookup_by_node_id(s2a, 999) == NULL);
        smap_s2a_free(s2a);
    }
    {
        SmapSourceToAst *s2a = smap_s2a_new();
        SmapPosition p0 = {"x.bf", 1, 1, 1}, p1 = {"x.bf", 1, 2, 1};
        smap_s2a_add(s2a, &p0, 0);
        smap_s2a_add(s2a, &p1, 1);
        ISO_CHECK(smap_s2a_lookup_by_node_id(s2a, 0)->column == 1);
        ISO_CHECK(smap_s2a_lookup_by_node_id(s2a, 1)->column == 2);
        smap_s2a_free(s2a);
    }

    /* ── AstToIr: add, forward and reverse lookup ───────────────────────── */
    {
        SmapAstToIr *a2i = smap_a2i_new();
        int64_t ids[] = {7, 8, 9, 10};
        ISO_CHECK(smap_a2i_add(a2i, 42, ids, 4) == 0);
        size_t n = 0;
        const int64_t *got = smap_a2i_lookup_by_ast_node_id(a2i, 42, &n);
        ISO_CHECK(got != NULL);
        ISO_CHECK_EQ_UINT(n, 4u);
        if (got && n == 4) ISO_CHECK_MEM_EQ(got, ids, 4 * sizeof(int64_t));

        size_t node = 999;
        ISO_CHECK(smap_a2i_lookup_by_ir_id(a2i, 8, &node) == 1);
        ISO_CHECK_EQ_UINT(node, 42u);
        ISO_CHECK(smap_a2i_lookup_by_ir_id(a2i, 99, &node) == 0);
        ISO_CHECK(smap_a2i_lookup_by_ast_node_id(a2i, 0, &n) == NULL);
        smap_a2i_free(a2i);
    }

    /* ── IrToIr: mappings, deletions, reverse lookup, pass name ─────────── */
    {
        SmapIrToIr *m = smap_i2i_new("contraction");
        int64_t hundred = 100;
        smap_i2i_add_mapping(m, 7, &hundred, 1);
        smap_i2i_add_mapping(m, 8, &hundred, 1);
        smap_i2i_add_mapping(m, 9, &hundred, 1);
        size_t n = 0;
        const int64_t *got = smap_i2i_lookup_by_original_id(m, 7, &n);
        ISO_CHECK(got != NULL && n == 1 && got[0] == 100);
        int64_t orig = 0;
        ISO_CHECK(smap_i2i_lookup_by_new_id(m, 100, &orig) == 1);
        ISO_CHECK(orig == 7); /* first one found */
        ISO_CHECK_STR_EQ(smap_i2i_pass_name(m), "contraction");
        smap_i2i_free(m);
    }
    {
        SmapIrToIr *m = smap_i2i_new("dead_store");
        smap_i2i_add_deletion(m, 5);
        ISO_CHECK(smap_i2i_is_deleted(m, 5));
        size_t n = 0;
        ISO_CHECK(smap_i2i_lookup_by_original_id(m, 5, &n) == NULL); /* deleted */
        int64_t orig;
        ISO_CHECK(smap_i2i_lookup_by_original_id(m, 0, &n) == NULL);
        ISO_CHECK(smap_i2i_lookup_by_new_id(m, 0, &orig) == 0);
        smap_i2i_free(m);
    }

    /* ── IrToMachineCode: id lookup and offset containment ──────────────── */
    {
        SmapIrToMc *mc = smap_i2mc_new();
        smap_i2mc_add(mc, 3, 0x14, 4); /* bytes 0x14..0x18 */
        size_t off = 0, len = 0;
        ISO_CHECK(smap_i2mc_lookup_by_ir_id(mc, 3, &off, &len) == 1);
        ISO_CHECK_EQ_UINT(off, 0x14u);
        ISO_CHECK_EQ_UINT(len, 4u);
        int64_t id = 0;
        ISO_CHECK(smap_i2mc_lookup_by_mc_offset(mc, 0x14, &id) == 1 && id == 3);
        ISO_CHECK(smap_i2mc_lookup_by_mc_offset(mc, 0x15, &id) == 1); /* inside */
        ISO_CHECK(smap_i2mc_lookup_by_mc_offset(mc, 0x17, &id) == 1); /* last byte */
        ISO_CHECK(smap_i2mc_lookup_by_mc_offset(mc, 0x18, &id) == 0); /* past end */
        ISO_CHECK(smap_i2mc_lookup_by_ir_id(mc, 0, &off, &len) == 0);
        smap_i2mc_free(mc);
    }

    /* ── chain: empty / no backend ──────────────────────────────────────── */
    {
        SmapChain *c = smap_chain_new();
        SmapMcEntry *res = NULL;
        size_t n = 0;
        SmapPosition p = {"x.bf", 1, 1, 1};
        ISO_CHECK(smap_chain_source_to_mc(c, &p, &res, &n) == 0);
        ISO_CHECK(smap_chain_mc_to_source(c, 0) == NULL);
        smap_chain_free(c);
    }

    /* ── chain: full end-to-end round-trip (no passes) ──────────────────── */
    {
        SmapChain *c = smap_chain_new();
        SmapPosition p = {"test.bf", 1, 1, 1};
        smap_s2a_add(smap_chain_source_to_ast(c), &p, 0);
        int64_t ids[] = {7, 8, 9, 10};
        smap_a2i_add(smap_chain_ast_to_ir(c), 0, ids, 4);
        SmapIrToMc *mc = smap_i2mc_new();
        smap_i2mc_add(mc, 7, 0, 4);
        smap_i2mc_add(mc, 8, 4, 4);
        smap_i2mc_add(mc, 9, 8, 4);
        smap_i2mc_add(mc, 10, 12, 4);
        smap_chain_set_machine_code(c, mc);

        SmapMcEntry *res = NULL;
        size_t n = 0;
        ISO_CHECK(smap_chain_source_to_mc(c, &p, &res, &n) == 1);
        ISO_CHECK_EQ_UINT(n, 4u);
        if (n == 4) ISO_CHECK_EQ_UINT(res[0].mc_offset, 0u);
        free(res);

        const SmapPosition *found = smap_chain_mc_to_source(c, 0);
        ISO_CHECK(found != NULL);
        if (found) {
            ISO_CHECK_STR_EQ(found->file, "test.bf");
            ISO_CHECK(found->line == 1 && found->column == 1);
        }
        smap_chain_free(c);
    }

    /* ── chain: an optimiser pass is followed forward ───────────────────── */
    {
        SmapChain *c = smap_chain_new();
        SmapPosition p = {"test.bf", 1, 1, 1};
        smap_s2a_add(smap_chain_source_to_ast(c), &p, 0);
        int64_t ids[] = {7, 8, 9};
        smap_a2i_add(smap_chain_ast_to_ir(c), 0, ids, 3);

        SmapIrToIr *pass = smap_i2i_new("contraction");
        int64_t hundred = 100;
        smap_i2i_add_mapping(pass, 7, &hundred, 1);
        smap_i2i_add_mapping(pass, 8, &hundred, 1);
        smap_i2i_add_mapping(pass, 9, &hundred, 1);
        smap_chain_add_optimizer_pass(c, pass);

        SmapIrToMc *mc = smap_i2mc_new();
        smap_i2mc_add(mc, 100, 0, 4);
        smap_chain_set_machine_code(c, mc);

        SmapMcEntry *res = NULL;
        size_t n = 0;
        ISO_CHECK(smap_chain_source_to_mc(c, &p, &res, &n) == 1);
        ISO_CHECK(n >= 1);            /* all three trace to 100 → (0,4) */
        if (n >= 1) ISO_CHECK_EQ_UINT(res[0].mc_offset, 0u);
        free(res);
        smap_chain_free(c);
    }

    /* ── chain: a deleted instruction is excluded ───────────────────────── */
    {
        SmapChain *c = smap_chain_new();
        SmapPosition p = {"test.bf", 1, 1, 1};
        smap_s2a_add(smap_chain_source_to_ast(c), &p, 0);
        int64_t five = 5;
        smap_a2i_add(smap_chain_ast_to_ir(c), 0, &five, 1);

        SmapIrToIr *pass = smap_i2i_new("dead_store");
        smap_i2i_add_deletion(pass, 5);
        smap_chain_add_optimizer_pass(c, pass);

        SmapIrToMc *mc = smap_i2mc_new();
        smap_i2mc_add(mc, 5, 0, 4);
        smap_chain_set_machine_code(c, mc);

        SmapMcEntry *res = NULL;
        size_t n = 0;
        ISO_CHECK(smap_chain_source_to_mc(c, &p, &res, &n) == 0); /* excluded */
        free(res);
        smap_chain_free(c);
    }

    return ISO_TEST_RESULT();
}
