/*
 * test_util.h -- helpers shared by the test executables (not part of the library).
 *
 * Plain (non-static) functions in test_util.c, so a test that does not use one
 * never trips -Wunused-function under -Werror.
 */
#ifndef TEST_UTIL_H
#define TEST_UTIL_H

#include <stddef.h>
#include <stdint.h>

#include "pbt.h"
#include "pbt_json.h"

/* Whole file into a NUL-terminated heap buffer (free with free()); NULL on failure. */
char *tu_read_file(const char *path, size_t *len);
/* The committed pbt-v1 cases.json, found by walking up from the working directory; NULL if absent. */
char *tu_read_corpus_text(size_t *len);
/* Any repository file by its repo-relative path (e.g. "code/specs/fixtures/fuzz-v1/cases.json"), found by walking up
 * from the working directory; NULL if absent. */
char *tu_read_repo_file(const char *relpath, size_t *len);
/* Parse the committed corpus; NULL on failure (the test then fails loudly). */
pbt_json *tu_load_corpus(void);

/* Parse "-123" style text into a big (test inputs are always valid). */
pbt_big tu_big(const char *dec);
/* A property from a plain function: 1 holds. */
typedef int (*tu_val_fn)(const pbt_val *v);
pbt_prop tu_prop(tu_val_fn fn);

/* The inverse of the SplitMix64 finalizer chain: the stream state whose NEXT output is `target`. */
uint64_t tu_state_for_output(uint64_t target);

/* JSON tree helpers for building corruptions (heap strings replaced in place). */
void tu_set_str(pbt_json *node, const char *text);
pbt_json *tu_case(pbt_json *doc, size_t index);
const char *tu_case_id(pbt_json *doc, size_t index);
/* Replace the "value" of the int object stored at `field` of a case. */
void tu_set_int_field(pbt_json *cs, const char *field, const char *dec);

/* True if any stored problem starts with "<id>: ". */
int tu_report_names(const pbt_corpus_report *r, const char *id);
/* True if any stored problem starts with an id of a DIFFERENT case of `doc`. */
int tu_report_names_other(const pbt_corpus_report *r, pbt_json *doc, const char *id);

#endif /* TEST_UTIL_H */
