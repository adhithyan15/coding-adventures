/* Tests for the C vcd-writer, using the header-only iso_test.h harness (pure
 * ISO). The main vector reproduces the Rust crate's documented example exactly;
 * the rest check scalar / real / skip-unchanged / dumpvars behaviour. */
#include "iso_test.h"

#include <string.h> /* strcmp, strstr, strlen */

#include "vcd_writer.h"

int main(void) {
    /* ── documented example: exact full-document match ──────────────────── */
    {
        VcdWriter *w = vcd_new("1ps");
        char a_id[16], sum_id[16];
        const char *want =
            "$date 2026-06-13 00:00:00 UTC $end\n"
            "$version Silicon-Stack VCD Writer 0.1.0 $end\n"
            "$timescale 1ps $end\n"
            "$scope module adder $end\n"
            "$var wire 4 ! a [3:0] $end\n"
            "$var wire 5 \" sum [4:0] $end\n"
            "$upscope $end\n"
            "$enddefinitions $end\n"
            "#0\n"
            "b0 !\n"
            "b0 \"\n"
            "#10\n"
            "b11 !\n"
            "b1000 \"\n";
        vcd_open_scope(w, "adder");
        ISO_CHECK(vcd_declare(w, "a", 4, "wire", a_id, sizeof a_id));
        ISO_CHECK(strcmp(a_id, "!") == 0);
        ISO_CHECK(vcd_declare(w, "sum", 5, "wire", sum_id, sizeof sum_id));
        ISO_CHECK(strcmp(sum_id, "\"") == 0);
        vcd_close_scope(w);
        vcd_end_definitions(w);
        vcd_time(w, 0);
        vcd_value_change(w, a_id, 0);
        vcd_value_change(w, sum_id, 0);
        vcd_time(w, 10);
        vcd_value_change(w, a_id, 3);
        vcd_value_change(w, sum_id, 8);
        ISO_CHECK(vcd_ok(w));
        ISO_CHECK(strcmp(vcd_text(w), want) == 0);
        vcd_free(w);
    }

    /* ── scalar (width 1) uses a single bit, no 'b' prefix ──────────────── */
    {
        VcdWriter *w = vcd_new("1ns");
        char clk[16];
        vcd_declare(w, "clk", 1, "wire", clk, sizeof clk);
        vcd_end_definitions(w);
        vcd_value_change_at(w, 0, clk, 0);
        vcd_value_change_at(w, 5, clk, 1);
        ISO_CHECK(strstr(vcd_text(w), "$var wire 1 ! clk $end\n") != NULL);
        ISO_CHECK(strstr(vcd_text(w), "\n0!\n") != NULL); /* clk=0 */
        ISO_CHECK(strstr(vcd_text(w), "\n1!\n") != NULL); /* clk=1 */
        vcd_free(w);
    }

    /* ── real values use the r<n> form ──────────────────────────────────── */
    {
        VcdWriter *w = vcd_new("1ps");
        char t[16];
        vcd_declare(w, "temp", 64, "real", t, sizeof t);
        vcd_end_definitions(w);
        vcd_value_change_at(w, 0, t, 42);
        ISO_CHECK(strstr(vcd_text(w), "r42 !\n") != NULL);
        vcd_free(w);
    }

    /* ── unchanged value is skipped ─────────────────────────────────────── */
    {
        VcdWriter *w = vcd_new("1ps");
        char v[16];
        const char *txt;
        const char *p;
        int count = 0;
        vcd_declare(w, "x", 4, "wire", v, sizeof v);
        vcd_end_definitions(w);
        vcd_time(w, 0);
        vcd_value_change(w, v, 5);
        vcd_value_change(w, v, 5); /* skipped */
        vcd_value_change(w, v, 5); /* skipped */
        txt = vcd_text(w);
        for (p = txt; (p = strstr(p, "b101 !")) != NULL; p++) {
            count++;
        }
        ISO_CHECK_EQ_INT(count, 1); /* emitted exactly once */
        vcd_free(w);
    }

    /* ── dump_initial emits every declared variable ─────────────────────── */
    {
        VcdWriter *w = vcd_new("1ps");
        char a[16], b[16];
        const char *ids[1];
        int64_t vals[1];
        vcd_declare(w, "a", 4, "wire", a, sizeof a);
        vcd_declare(w, "b", 4, "wire", b, sizeof b);
        vcd_end_definitions(w);
        ids[0] = a;
        vals[0] = 7;
        vcd_dump_initial(w, ids, vals, 1);
        ISO_CHECK(strstr(vcd_text(w), "$dumpvars\n") != NULL);
        ISO_CHECK(strstr(vcd_text(w), "b111 !\n") != NULL); /* a = 7 override */
        ISO_CHECK(strstr(vcd_text(w), "b0 \"\n") != NULL);  /* b = 0 default */
        ISO_CHECK(strstr(vcd_text(w), "$end\n") != NULL);
        vcd_free(w);
    }

    /* ── two-character identifiers after 94 single-char ones ────────────── */
    {
        VcdWriter *w = vcd_new("1ps");
        char id[16];
        int i;
        int ok;
        for (i = 0; i < 94; i++) {
            vcd_declare(w, "s", 1, "wire", id, sizeof id);
        }
        /* the 95th (index 94) is the first two-character id. */
        ok = vcd_declare(w, "s", 1, "wire", id, sizeof id);
        ISO_CHECK(ok && strlen(id) == 2);
        ISO_CHECK(strcmp(id, "!!") == 0);
        vcd_free(w);
    }

    return ISO_TEST_RESULT();
}
