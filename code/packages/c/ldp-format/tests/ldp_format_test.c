/*
 * Tests for ldp-format, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's unit tests. The "original" files are built on
 * the stack with string literals (never freed via ldp_file_free); only the
 * ldp_read output is owned and freed.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "ldp_format.h"

/* Build a minimal empty file (header only). */
static LdpFile empty_file(void) {
    LdpFile f;
    memset(&f, 0, sizeof f);
    f.version_major = 1;
    f.version_minor = 0;
    f.language = (char *)"twig";
    f.flags = 0;
    return f;
}

/* Round-trip an in-memory file: write, read back, compare, free the read copy.
 * Returns 1 on a successful equal round-trip. */
static int round_trips(const LdpFile *original) {
    uint8_t *bytes = NULL;
    size_t len = 0;
    LdpFile *restored = NULL;
    int ok;
    if (ldp_write(original, &bytes, &len) != LDP_OK) {
        return 0;
    }
    if (ldp_read(bytes, len, &restored) != LDP_OK) {
        free(bytes);
        return 0;
    }
    ok = ldp_file_equal(original, restored);
    ldp_file_free(restored);
    free(bytes);
    return ok;
}

int main(void) {
    /* ── empty round-trip ──────────────────────────────────────────────────*/
    {
        LdpFile f = empty_file();
        ISO_CHECK(round_trips(&f));
    }

    /* ── rich round-trip ───────────────────────────────────────────────────*/
    {
        LdpTypeSeen fact_i0_types[] = {{(char *)"int", 1000000}};
        LdpTypeSeen fact_i1_types[] = {{(char *)"int", 800000},
                                       {(char *)"nil", 199999}};
        LdpInstruction fact_instrs[2];
        char *fact_params[] = {(char *)"int"};
        char *decode_params[] = {(char *)"int", (char *)"int"};
        LdpFunction main_mod_fns[2];
        LdpFunction another_mod_fns[1];
        LdpModule modules[2];
        LdpFile f = empty_file();
        f.flags = 3;

        memset(fact_instrs, 0, sizeof fact_instrs);
        fact_instrs[0].instr_index = 0;
        fact_instrs[0].opcode = (char *)"const";
        fact_instrs[0].observation_count = 1000000;
        fact_instrs[0].observed_kind = LDP_MONO;
        fact_instrs[0].observation_count_at_promotion = 100;
        fact_instrs[0].time_to_first_observation_ns = 1000;
        fact_instrs[0].time_to_promotion_ns = 122000000;
        fact_instrs[0].types_seen = fact_i0_types;
        fact_instrs[0].types_seen_len = 1;
        fact_instrs[1].instr_index = 5;
        fact_instrs[1].opcode = (char *)"call_builtin";
        fact_instrs[1].observation_count = 999999;
        fact_instrs[1].observed_kind = LDP_POLY;
        fact_instrs[1].observation_count_at_promotion = 100;
        fact_instrs[1].time_to_first_observation_ns = 2000;
        fact_instrs[1].time_to_promotion_ns = 122000000;
        fact_instrs[1].types_seen = fact_i1_types;
        fact_instrs[1].types_seen_len = 2;

        memset(main_mod_fns, 0, sizeof main_mod_fns);
        main_mod_fns[0].name = (char *)"fact";
        main_mod_fns[0].params = fact_params;
        main_mod_fns[0].params_len = 1;
        main_mod_fns[0].call_count = 1000000;
        main_mod_fns[0].total_self_time_ns = 5000000000ull;
        main_mod_fns[0].type_status = LDP_UNTYPED;
        main_mod_fns[0].promotion_state = LDP_JITTED;
        main_mod_fns[0].instructions = fact_instrs;
        main_mod_fns[0].instructions_len = 2;
        main_mod_fns[1].name = (char *)"main";
        main_mod_fns[1].call_count = 1;
        main_mod_fns[1].total_self_time_ns = 6000000000ull;
        main_mod_fns[1].type_status = LDP_UNTYPED;
        main_mod_fns[1].promotion_state = LDP_INTERP;

        memset(another_mod_fns, 0, sizeof another_mod_fns);
        another_mod_fns[0].name = (char *)"decode";
        another_mod_fns[0].params = decode_params;
        another_mod_fns[0].params_len = 2;
        another_mod_fns[0].type_status = LDP_PARTIALLY_TYPED;
        another_mod_fns[0].promotion_state = LDP_DEOPTED;

        memset(modules, 0, sizeof modules);
        modules[0].name = (char *)"main_mod";
        modules[0].functions = main_mod_fns;
        modules[0].functions_len = 2;
        modules[1].name = (char *)"another_mod";
        modules[1].functions = another_mod_fns;
        modules[1].functions_len = 1;
        f.modules = modules;
        f.modules_len = 2;

        ISO_CHECK(round_trips(&f));

        /* determinism: two writes are byte-identical */
        {
            uint8_t *a = NULL, *b = NULL;
            size_t la = 0, lb = 0;
            ISO_CHECK_EQ_INT(ldp_write(&f, &a, &la), LDP_OK);
            ISO_CHECK_EQ_INT(ldp_write(&f, &b, &lb), LDP_OK);
            ISO_CHECK_EQ_UINT(la, lb);
            ISO_CHECK(la > 0 && memcmp(a, b, la) == 0);
            free(a);
            free(b);
        }
    }

    /* ── string-table dedup keeps size small ───────────────────────────────*/
    {
        enum { N = 100 };
        LdpTypeSeen ts[N];
        LdpInstruction instrs[N];
        char *params[N][2];
        LdpFunction fns[N];
        LdpModule mods[N];
        char names[N][16];
        LdpFile f = empty_file();
        uint8_t *bytes = NULL;
        size_t len = 0, i;
        LdpFile *restored = NULL;

        for (i = 0; i < N; i++) {
            ts[i].type_name = (char *)"int";
            ts[i].count = 1;
            memset(&instrs[i], 0, sizeof instrs[i]);
            instrs[i].opcode = (char *)"const";
            instrs[i].observation_count = 1;
            instrs[i].observed_kind = LDP_MONO;
            instrs[i].types_seen = &ts[i];
            instrs[i].types_seen_len = 1;
            params[i][0] = (char *)"int";
            params[i][1] = (char *)"bool";
            memset(&fns[i], 0, sizeof fns[i]);
            /* fn_<i> unique names; sprintf-free integer formatting */
            {
                char *p = names[i];
                size_t v = i, k = 0;
                char tmp[8];
                p[k++] = 'f';
                p[k++] = 'n';
                p[k++] = '_';
                if (v == 0) {
                    tmp[0] = '0';
                    tmp[1] = '\0';
                } else {
                    size_t t = 0;
                    while (v > 0) {
                        tmp[t++] = (char)('0' + (v % 10));
                        v /= 10;
                    }
                    tmp[t] = '\0';
                    /* reverse */
                    {
                        size_t a = 0, b = t - 1;
                        while (a < b) {
                            char c = tmp[a];
                            tmp[a] = tmp[b];
                            tmp[b] = c;
                            a++;
                            b--;
                        }
                    }
                }
                {
                    size_t j = 0;
                    while (tmp[j]) {
                        p[k++] = tmp[j++];
                    }
                    p[k] = '\0';
                }
            }
            fns[i].name = names[i];
            fns[i].params = params[i];
            fns[i].params_len = 2;
            fns[i].call_count = i;
            fns[i].instructions = &instrs[i];
            fns[i].instructions_len = 1;
            memset(&mods[i], 0, sizeof mods[i]);
            mods[i].name = (char *)"shared_module";
            mods[i].functions = &fns[i];
            mods[i].functions_len = 1;
        }
        f.modules = mods;
        f.modules_len = N;

        ISO_CHECK_EQ_INT(ldp_write(&f, &bytes, &len), LDP_OK);
        ISO_CHECK_EQ_INT(ldp_read(bytes, len, &restored), LDP_OK);
        ISO_CHECK_EQ_UINT(restored->modules_len, (size_t)N);
        ISO_CHECK(len / N < 150); /* dedup working */
        ldp_file_free(restored);
        free(bytes);
    }

    /* ── reject bad magic ──────────────────────────────────────────────────*/
    {
        uint8_t bad[32];
        LdpFile *f = NULL;
        memset(bad, 0, sizeof bad);
        bad[0] = 'B';
        bad[1] = 'A';
        bad[2] = 'D';
        ISO_CHECK_EQ_INT(ldp_read(bad, sizeof bad, &f), LDP_ERR_BAD_MAGIC);
        ISO_CHECK(f == NULL);
    }

    /* ── reject unsupported major version ──────────────────────────────────*/
    {
        uint8_t b[32];
        LdpFile *f = NULL;
        memset(b, 0, sizeof b);
        b[0] = 'L';
        b[1] = 'D';
        b[2] = 'P';
        b[3] = 0;
        b[4] = 99; /* version_major low byte */
        ISO_CHECK_EQ_INT(ldp_read(b, sizeof b, &f),
                         LDP_ERR_UNSUPPORTED_MAJOR);
    }

    /* ── reject truncated input ────────────────────────────────────────────*/
    {
        LdpFile f = empty_file();
        uint8_t *bytes = NULL;
        size_t len = 0;
        LdpFile *restored = NULL;
        ISO_CHECK_EQ_INT(ldp_write(&f, &bytes, &len), LDP_OK);
        ISO_CHECK_EQ_INT(ldp_read(bytes, len / 2, &restored),
                         LDP_ERR_UNEXPECTED_EOF);
        free(bytes);
    }

    /* ── language validation on write ──────────────────────────────────────*/
    {
        LdpFile f = empty_file();
        uint8_t *bytes = NULL;
        size_t len = 0;
        f.language = (char *)"xxxxxxxxxxxxxxxxx"; /* 17 chars */
        ISO_CHECK_EQ_INT(ldp_write(&f, &bytes, &len),
                         LDP_ERR_LANGUAGE_TOO_LONG);
        f.language = (char *)"tw\xC9\xA1g"; /* non-ASCII */
        ISO_CHECK_EQ_INT(ldp_write(&f, &bytes, &len),
                         LDP_ERR_LANGUAGE_NOT_ASCII);
    }

    /* ── unicode in module/function names round-trips ──────────────────────*/
    {
        char *params[] = {(char *)"int"};
        LdpFunction fn;
        LdpModule m;
        LdpFile f = empty_file();
        memset(&fn, 0, sizeof fn);
        fn.name = (char *)"na\xC3\xAFve_decode";
        fn.params = params;
        fn.params_len = 1;
        memset(&m, 0, sizeof m);
        m.name = (char *)"\xE3\x83\xA2\xE3\x82\xB8\xE3\x83\xA5";
        m.functions = &fn;
        m.functions_len = 1;
        f.modules = &m;
        f.modules_len = 1;
        ISO_CHECK(round_trips(&f));
    }

    /* ── coverage: every observed_kind / type_status / promotion_state ─────*/
    {
        int k, tsv, psv;
        for (k = 0; k <= 3; k++) {
            LdpInstruction ins;
            LdpFunction fn;
            LdpModule m;
            LdpFile f = empty_file();
            uint8_t *bytes = NULL;
            size_t len = 0;
            LdpFile *restored = NULL;
            memset(&ins, 0, sizeof ins);
            ins.opcode = (char *)"const";
            ins.observation_count = 1;
            ins.observed_kind = (LdpObservedKind)k;
            memset(&fn, 0, sizeof fn);
            fn.name = (char *)"f";
            fn.call_count = 1;
            fn.instructions = &ins;
            fn.instructions_len = 1;
            memset(&m, 0, sizeof m);
            m.name = (char *)"m";
            m.functions = &fn;
            m.functions_len = 1;
            f.modules = &m;
            f.modules_len = 1;
            ISO_CHECK_EQ_INT(ldp_write(&f, &bytes, &len), LDP_OK);
            ISO_CHECK_EQ_INT(ldp_read(bytes, len, &restored), LDP_OK);
            ISO_CHECK_EQ_INT(
                restored->modules[0].functions[0].instructions[0].observed_kind,
                k);
            ldp_file_free(restored);
            free(bytes);
        }
        for (tsv = 0; tsv <= 2; tsv++) {
            for (psv = 0; psv <= 2; psv++) {
                LdpFunction fn;
                LdpModule m;
                LdpFile f = empty_file();
                uint8_t *bytes = NULL;
                size_t len = 0;
                LdpFile *restored = NULL;
                memset(&fn, 0, sizeof fn);
                fn.name = (char *)"f";
                fn.type_status = (LdpTypeStatus)tsv;
                fn.promotion_state = (LdpPromotionState)psv;
                memset(&m, 0, sizeof m);
                m.name = (char *)"m";
                m.functions = &fn;
                m.functions_len = 1;
                f.modules = &m;
                f.modules_len = 1;
                ISO_CHECK_EQ_INT(ldp_write(&f, &bytes, &len), LDP_OK);
                ISO_CHECK_EQ_INT(ldp_read(bytes, len, &restored), LDP_OK);
                ISO_CHECK_EQ_INT(restored->modules[0].functions[0].type_status,
                                 tsv);
                ISO_CHECK_EQ_INT(
                    restored->modules[0].functions[0].promotion_state, psv);
                ldp_file_free(restored);
                free(bytes);
            }
        }
    }

    return ISO_TEST_RESULT();
}
