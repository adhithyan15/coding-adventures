/*
 * smoke.c — exercise the spreadsheet C ABI from real C through the built
 * shared library, asserting the same results the Rust/WASM/TS engines produce.
 * Compiled and run by build-capi.sh. Exits non-zero on any mismatch.
 */
#include "spreadsheet.h"
#include <stdio.h>
#include <string.h>

static int failures = 0;

static void check(const char *label, const char *got, const char *needle) {
    int ok = got && strstr(got, needle) != NULL;
    if (!ok) failures++;
    printf("%s  %s: %s\n", ok ? "ok  " : "FAIL", label, got ? got : "(null)");
}

int main(void) {
    ScSession *s = sc_session_new();

    const char *seed[][2] = {
        {"B1", "15"}, {"B2", "8"}, {"B3", "12"}, {"B4", "4"}, {"B5", "7"},
    };
    for (int i = 0; i < 5; i++) {
        sc_string_free(sc_set_cell(s, seed[i][0], seed[i][1]));
    }
    sc_string_free(sc_set_cell(s, "B6", "=SUM(B1:B5)"));
    sc_string_free(sc_set_cell(s, "B7", "=AVERAGE(B1:B5)"));
    sc_string_free(sc_set_cell(s, "C1", "=1/0"));

    char *v;
    v = sc_get_value(s, "B6");  check("B6 SUM",        v, "\"value\":46");  sc_string_free(v);
    v = sc_get_value(s, "B7");  check("B7 AVERAGE",    v, "\"value\":9.2"); sc_string_free(v);
    v = sc_get_raw(s,   "B6");  check("B6 raw",        v, "=SUM(B1:B5)");   sc_string_free(v);
    v = sc_get_value(s, "C1");  check("C1 div-by-0",   v, "#DIV/0!");       sc_string_free(v);

    /* Incremental recalc. */
    sc_string_free(sc_set_cell(s, "B1", "115"));
    v = sc_get_value(s, "B6");  check("B6 after edit", v, "\"value\":146"); sc_string_free(v);

    sc_session_free(s);

    printf(failures == 0 ? "\nALL PASS\n" : "\n%d FAILURE(S)\n", failures);
    return failures == 0 ? 0 : 1;
}
