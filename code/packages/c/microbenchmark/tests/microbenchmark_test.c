#if defined(_MSC_VER)
/* Use ISO stdio in this test rather than the nonportable fopen_s variant. */
#define _CRT_SECURE_NO_WARNINGS
#endif
#include "bounded_json.h"
#include "iso_test.h"
#include "microbenchmark.h"
#include "pbt.h"
#include "rank_statistics.h"
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
typedef struct {
  int64_t now, step;
  int fail, calls, regression_at;
} FakeClock;
typedef struct {
  int64_t each;
  int fail;
} Batch;
typedef struct {
  int64_t each;
  unsigned calls, fail_at, wrong_checksum_at;
} SequencedBatch;
typedef struct {
  const int64_t *ticks;
  size_t count, index;
} TickSequence;
typedef struct {
  int fail, fail_at, allocations, releases;
} AllocationState;
static void *test_allocate(size_t bytes, void *context) {
  AllocationState *state = (AllocationState *)context;
  state->allocations++;
  return state->fail || state->fail_at == state->allocations ? NULL
                                                             : malloc(bytes);
}
static void test_release(void *pointer, void *context) {
  AllocationState *state = (AllocationState *)context;
  state->releases++;
  free(pointer);
}
static int fake_clock(void *p, int64_t *out) {
  FakeClock *c = (FakeClock *)p;
  if (c->fail)
    return 0;
  c->now += c->step;
  c->calls++;
  if (c->regression_at == c->calls)
    c->now -= c->step + 1;
  *out = c->now;
  return 1;
}
static int batch(void *p, size_t n, int64_t *out) {
  Batch *b = (Batch *)p;
  int64_t count = (int64_t)n;
  if (b->fail)
    return 0;
  if (b->each > 0 && count > INT64_MAX / b->each)
    return 0;
  if (b->each < 0) {
    uint64_t mag = (uint64_t)(-(b->each + 1)) + 1;
    if ((uint64_t)count > (UINT64_C(1) << 63) / mag)
      return 0;
  }
  *out = b->each * count;
  return 1;
}
static int sequenced_batch(void *p, size_t n, int64_t *out) {
  SequencedBatch *b = (SequencedBatch *)p;
  b->calls++;
  if (b->calls == b->fail_at)
    return 0;
  *out = b->each * (int64_t)n;
  if (b->calls == b->wrong_checksum_at)
    *out += 1;
  return 1;
}
static int sequence_clock(void *p, int64_t *out) {
  TickSequence *sequence = (TickSequence *)p;
  if (sequence->index >= sequence->count)
    return 0;
  *out = sequence->ticks[sequence->index++];
  return 1;
}
static int prop_interval(void *user, pbt_budget *scope, const pbt_val *value) {
  int64_t n;
  double sample[64];
  RSInterval ci;
  size_t i;
  (void)user;
  (void)scope;
  if (value->kind != PBT_V_INT || !pbt_big_fits_i64(&value->u.i))
    return 0;
  n = pbt_big_to_i64(&value->u.i);
  if (n < 1 || n > 64)
    return 0;
  for (i = 0; i < (size_t)n; i++)
    sample[i] = (double)i;
  if (rs_median_interval(sample, (size_t)n, 95, &ci) != RS_OK)
    return 0;
  return !ci.available ||
         (ci.low <= ci.high && ci.low == sample[(size_t)(ci.low)] &&
          ci.high == sample[(size_t)(ci.high)]);
}
static void test_generated(void) {
  pbt_gen *g = pbt_gen_int_i64(1, 64);
  pbt_prop p = {prop_interval, NULL};
  pbt_options o = {80, 100, PBT_CI_OFF};
  pbt_verdict v;
  pbt_verdict_init(&v);
  ISO_CHECK(g != NULL);
  if (g) {
    ISO_CHECK(pbt_check(NULL, UINT64_C(0xC0175EED), g, p, &o, NULL, &v) == 0);
    ISO_CHECK(v.passed);
    pbt_gen_release(g);
  }
  pbt_verdict_free(&v);
}
static void test_fixture_cases(void) {
  MBPair p[9];
  MBAnalysis a;
  size_t i;
  int64_t candidates[] = {80,  80,  80,  80,  80,  80,  120, 120, 120, 120,
                          120, 120, 256, 256, 272, 272, 289, 289, 85,  90,
                          95,  105, 110, 115, 50,  50,  50,  50,  50,  300,
                          82,  80,  10,  81,  83,  84,  85,  86};
  int64_t baselines[] = {100, 100, 272, 100, 100, 100};
  size_t ns[] = {6, 6, 6, 6, 5, 9};
  double ratios[] = {.8, 1.2, 1.0, 1.0, .5, .83};
  MBVerdict verdicts[] = {MB_FASTER,       MB_SLOWER,       MB_EQUIVALENT,
                          MB_INCONCLUSIVE, MB_INCONCLUSIVE, MB_FASTER};
  size_t offset = 0;
  for (i = 0; i < 6; i++) {
    size_t j;
    for (j = 0; j < ns[i]; j++) {
      p[j].baseline_ns = baselines[i];
      p[j].candidate_ns = candidates[offset + j];
      p[j].order =
          (j < ((ns[i] + 1) / 2)) ? MB_BASELINE_FIRST : MB_CANDIDATE_FIRST;
    }
    offset += ns[i];
    ISO_CHECK(mb_analyze(p, ns[i], 95, 1.0625, &a) == MB_OK);
    ISO_CHECK_EQ_DBL(a.ratio, ratios[i], 1e-14);
    ISO_CHECK(a.verdict == verdicts[i]);
  }
}
static const BJsonValue *fixture_member(const BJsonValue *v, const char *key) {
  size_t i, n;
  const char *name;
  size_t len;
  const BJsonValue *value;
  n = bj_size(v);
  for (i = 0; i < n; i++)
    if (bj_member(v, i, &name, &len, &value) && len == strlen(key) &&
        memcmp(name, key, len) == 0)
      return value;
  return NULL;
}
static void test_shared_fixture(void) {
  FILE *f = fopen("testdata/microbenchmark-pairs-v1.json", "rb");
  long length;
  char *bytes;
  BJsonDoc *doc = NULL;
  BJsonError error;
  const BJsonValue *root, *cases;
  size_t i;
  double factor = 0;
  int64_t confidence = 0;
  if (!f) {
    ISO_CHECK(0);
    return;
  }
  if (fseek(f, 0, SEEK_END) != 0 || (length = ftell(f)) < 0 ||
      length > (long)BJ_MAX_BYTES || fseek(f, 0, SEEK_SET) != 0) {
    fclose(f);
    ISO_CHECK(0);
    return;
  }
  bytes = (char *)malloc((size_t)length);
  if (!bytes) {
    fclose(f);
    ISO_CHECK(0);
    return;
  }
  if (fread(bytes, 1, (size_t)length, f) != (size_t)length) {
    fclose(f);
    free(bytes);
    ISO_CHECK(0);
    return;
  }
  fclose(f);
  ISO_CHECK(bj_parse(bytes, (size_t)length, &doc, &error) == BJ_OK);
  free(bytes);
  if (!doc)
    return;
  root = bj_root(doc);
  cases = fixture_member(root, "cases");
  ISO_CHECK(bj_kind(cases) == BJ_ARRAY);
  if (bj_kind(cases) != BJ_ARRAY) {
    bj_doc_free(&doc);
    return;
  }
  ISO_CHECK(bj_number(fixture_member(root, "minimum_factor"), &factor));
  ISO_CHECK(
      bj_integer(fixture_member(root, "confidence_percent"), &confidence));
  for (i = 0; i < bj_size(cases); i++) {
    const BJsonValue *item = bj_at(cases, i),
                     *cand = fixture_member(item, "candidate_ns"),
                     *interval = fixture_member(item, "interval");
    size_t j, n = bj_size(cand);
    MBPair pair[16];
    MBAnalysis got;
    double expected = 0;
    const char *verdict = NULL;
    size_t vl = 0;
    int64_t baseline = 0;
    ISO_CHECK(n <= 16);
    if (n > 16)
      continue;
    ISO_CHECK(bj_integer(fixture_member(item, "baseline_ns"), &baseline));
    for (j = 0; j < n; j++) {
      int64_t cns = 0;
      ISO_CHECK(bj_integer(bj_at(cand, j), &cns));
      pair[j].baseline_ns = baseline;
      pair[j].candidate_ns = cns;
      pair[j].order = j < (n + 1) / 2 ? MB_BASELINE_FIRST : MB_CANDIDATE_FIRST;
    }
    ISO_CHECK(mb_analyze(pair, n, (unsigned)confidence, factor, &got) == MB_OK);
    ISO_CHECK(bj_number(fixture_member(item, "ratio"), &expected));
    ISO_CHECK_EQ_DBL(got.ratio, expected, 1e-14);
    ISO_CHECK(bj_string(fixture_member(item, "verdict"), &verdict, &vl));
    ISO_CHECK(vl == strlen(mb_verdict_name(got.verdict)) &&
              memcmp(verdict, mb_verdict_name(got.verdict), vl) == 0);
    if (bj_kind(interval) == BJ_NULL)
      ISO_CHECK(!got.interval_available);
    else {
      double low = 0, high = 0;
      int valid = got.interval_available &&
                  bj_number(bj_at(interval, 0), &low) &&
                  bj_number(bj_at(interval, 1), &high);
      ISO_CHECK(valid);
      if (valid) {
        ISO_CHECK_EQ_DBL(got.low, low, 1e-14);
        ISO_CHECK_EQ_DBL(got.high, high, 1e-14);
      }
    }
  }
  bj_doc_free(&doc);
}
static int reject_replay(const char *json) {
  MBReport report;
  MBStatus status = mb_replay_json(json, strlen(json), &report);
  if (status == MB_OK)
    mb_report_free(&report);
  return status == MB_JSON && report.pairs == NULL;
}
static int replace_once(const char *source, const char *needle,
                        const char *replacement, char *out, size_t capacity) {
  const char *found = strstr(source, needle);
  size_t before, inserted, after;
  if (!found)
    return 0;
  before = (size_t)(found - source);
  inserted = strlen(replacement);
  after = strlen(found + strlen(needle));
  if (before > capacity || inserted > capacity - before ||
      after >= capacity - before - inserted)
    return 0;
  memcpy(out, source, before);
  memcpy(out + before, replacement, inserted);
  memcpy(out + before + inserted, found + strlen(needle), after + 1);
  return 1;
}
static void test_replay_schema_mutations(void) {
  static const char valid[] =
      "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[],"
      "\"revision\":\"r\",\"runtime\":\"C17\",\"os\":\"test\","
      "\"arch\":\"test\",\"config\":{\"pairs\":1,"
      "\"confidence_percent\":95,\"minimum_factor\":1.05,\"seed\":0,"
      "\"target_ns\":1,\"max_batch\":1},\"batch_size\":1,"
      "\"pairs\":[{\"baseline_ns\":1,\"candidate_ns\":1,"
      "\"order\":\"baseline-first\"}],\"analysis\":{\"ratio\":1,"
      "\"interval\":null,\"verdict\":\"inconclusive\"}}";
  static const struct {
    const char *needle;
    const char *replacement;
  } mutations[] = {
      {"\"revision\":\"r\"", "\"revision\":\"r\",\"revision\":\"other\""},
      {"\"runtime\":\"C17\"", "\"runtime\":\"C17\",\"profile\":\"release\",\"profile\":\"release\""},
      {"\"revision\":\"r\"", "\"profile\":\"release\""},
      {"\"revision\":\"r\"", "\"extra\":0,\"revision\":\"r\""},
      {"\"runtime\":\"C17\"", "\"runtime\":null"},
      {"\"invocation\":[]", "\"invocation\":null"},
      {"\"invocation\":[]", "\"invocation\":[0]"},
      {"\"minimum_factor\":1.05", "\"minimum_factor\":1.05,\"x\":0"},
      {"\"pairs\":1", "\"pairs\":10001"},
      {"\"confidence_percent\":95", "\"confidence_percent\":0"},
      {"\"minimum_factor\":1.05", "\"minimum_factor\":1"},
      {"\"target_ns\":1", "\"target_ns\":0"},
      {"\"max_batch\":1", "\"max_batch\":0"},
      {"\"batch_size\":1", "\"batch_size\":2"},
      {"\"baseline_ns\":1", "\"baseline_ns\":0"},
      {"\"candidate_ns\":1", "\"candidate_ns\":-1"},
      {"\"order\":\"baseline-first\"", "\"order\":\"sideways\""},
      {"\"order\":\"baseline-first\"", "\"order\":0"},
      {"\"analysis\":{", "\"analysis\":null"},
      {"\"ratio\":1", "\"ratio\":2"},
      {"\"interval\":null", "\"interval\":[0,2]"},
      {"\"verdict\":\"inconclusive\"", "\"verdict\":\"faster\""},
      {"\"seed\":0", "\"seed\":9223372036854775808"},
      {"\"runtime\":\"C17\"", "\"runtime\":\"C17\",\"profile\":null"},
      {"\"pairs\":[{", "\"pairs\":[{\"unexpected\":true,"},
      {"\"pairs\":[{", "\"pairs\":[]"},
  };
  static const char valid_interval[] =
      "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[],"
      "\"revision\":\"r\",\"runtime\":\"C17\",\"os\":\"test\","
      "\"arch\":\"test\",\"profile\":\"release\",\"config\":{"
      "\"pairs\":6,\"confidence_percent\":95,\"minimum_factor\":1.05,"
      "\"seed\":0,\"target_ns\":1,\"max_batch\":1},\"batch_size\":1,"
      "\"pairs\":[{\"baseline_ns\":100,\"candidate_ns\":100,"
      "\"order\":\"baseline-first\"},{\"baseline_ns\":100,"
      "\"candidate_ns\":100,\"order\":\"baseline-first\"},{"
      "\"baseline_ns\":100,\"candidate_ns\":100,"
      "\"order\":\"baseline-first\"},{\"baseline_ns\":100,"
      "\"candidate_ns\":100,\"order\":\"candidate-first\"},{"
      "\"baseline_ns\":100,\"candidate_ns\":100,"
      "\"order\":\"candidate-first\"},{\"baseline_ns\":100,"
      "\"candidate_ns\":100,\"order\":\"candidate-first\"}],"
      "\"analysis\":{\"ratio\":1,\"interval\":[1,1],"
      "\"verdict\":\"equivalent\"}}";
  size_t i;
  MBReport report;
  ISO_CHECK(mb_replay_json(valid, sizeof(valid) - 1, &report) == MB_OK);
  mb_report_free(&report);
  ISO_CHECK(mb_replay_json(valid_interval, sizeof(valid_interval) - 1,
                           &report) == MB_OK);
  ISO_CHECK(report.analysis.interval_available && report.analysis.low == 1.0 &&
            report.analysis.high == 1.0);
  mb_report_free(&report);
  ISO_CHECK(reject_replay("[]"));
  for (i = 0; i < sizeof(mutations) / sizeof(mutations[0]); i++) {
    char changed[2048];
    AllocationState allocation = {0, 0, 0, 0};
    MBAllocator allocator = {test_allocate, test_release, &allocation};
    MBReport rejected;
    ISO_CHECK(replace_once(valid, mutations[i].needle, mutations[i].replacement,
                           changed, sizeof(changed)));
    ISO_CHECK(reject_replay(changed));
    ISO_CHECK(mb_replay_json_with_allocator(changed, strlen(changed),
                                            &allocator, &rejected) == MB_JSON);
    ISO_CHECK(rejected.pairs == NULL && rejected.pair_count == 0);
    ISO_CHECK(allocation.allocations == allocation.releases);
  }
  {
    char changed[2048];
    ISO_CHECK(replace_once(valid_interval, "\"interval\":[1,1]",
                           "\"interval\":[0.5,1]", changed,
                           sizeof(changed)));
    ISO_CHECK(reject_replay(changed));
    ISO_CHECK(replace_once(valid_interval, "\"interval\":[1,1]",
                           "\"interval\":[1,2]", changed,
                           sizeof(changed)));
    ISO_CHECK(reject_replay(changed));
    ISO_CHECK(replace_once(valid_interval, "\"profile\":\"release\"",
                           "\"profile\":null", changed,
                           sizeof(changed)));
    ISO_CHECK(reject_replay(changed));
  }
}
static void test_portable_core_report_metadata(void) {
  static const char core[] =
      "{\"config\":{\"pairs\":1,\"confidence_percent\":95,"
      "\"minimum_factor\":1.05,\"seed\":0,\"target_ns\":1,"
      "\"max_batch\":1},\"batch_size\":1,\"pairs\":[{"
      "\"baseline_ns\":100,\"candidate_ns\":100,\"order\":"
      "\"baseline-first\"}],\"analysis\":{\"ratio\":1,"
      "\"interval\":null,\"verdict\":\"inconclusive\"}}";
  static const char complete[] =
      "{\"workload\":\"guest-tree-walk-8\",\"invocation\":[],"
      "\"revision\":\"r\",\"runtime\":\"C17\",\"os\":\"test\","
      "\"arch\":\"test\",\"config\":{\"pairs\":1,"
      "\"confidence_percent\":95,\"minimum_factor\":1.05,"
      "\"seed\":0,\"target_ns\":1,\"max_batch\":1},"
      "\"batch_size\":1,\"pairs\":[{\"baseline_ns\":100,"
      "\"candidate_ns\":100,\"order\":\"baseline-first\"}],"
      "\"analysis\":{\"ratio\":1,\"interval\":null,"
      "\"verdict\":\"inconclusive\"}}";
  static const struct {
    const char *source;
    const char *needle;
    const char *replacement;
  } invalid[] = {
      {complete, "\"invocation\":[],", ""},
      {core, "\"config\":", "\"profile\":\"release\",\"config\":"},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":null"},
      {complete, "\"runtime\":\"C17\"", "\"runtime\":null"},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":\"\""},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":\"   \""},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":\" \\t\\n\\r\\f\\u000b\""},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":\"\\u0000\""},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":\"bad\\u0000id\""},
      {complete, "\"runtime\":\"C17\"", "\"runtime\":\"\""},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":\"\xc3(\""},
      {complete, "\"invocation\":[]", "\"invocation\":[\"bad\\u0000arg\"]"},
      {complete, "\"invocation\":[]", "\"invocation\":null"},
      {complete, "\"workload\":\"guest-tree-walk-8\"",
       "\"workload\":17"},
      {complete, "\"arch\":\"test\"",
       "\"arch\":\"test\",\"profile\":\"\""},
      {complete, "\"arch\":\"test\"", "\"arch\":\"test\",\"other\":0"},
      {complete, "\"revision\":\"r\"", "\"revision\":\"r\",\"revision\":\"s\""},
      {core, "\"analysis\":{\"ratio\":1", "\"analysis\":{\"ratio\":2"},
      {core, "\"verdict\":\"inconclusive\"", "\"verdict\":\"faster\""},
  };
  MBReport report;
  size_t i;
  ISO_CHECK(mb_replay_json(core, sizeof(core) - 1, &report) == MB_OK);
  ISO_CHECK(report.pair_count == 1 && report.analysis.verdict == MB_INCONCLUSIVE);
  mb_report_free(&report);
  ISO_CHECK(mb_replay_json(complete, sizeof(complete) - 1, &report) == MB_OK);
  mb_report_free(&report);
  {
    char empty_argument[1024];
    ISO_CHECK(replace_once(complete, "\"invocation\":[]",
                           "\"invocation\":[\"\"]", empty_argument,
                           sizeof(empty_argument)));
    ISO_CHECK(mb_replay_json(empty_argument, strlen(empty_argument), &report) == MB_OK);
    mb_report_free(&report);
  }
  {
    char profiled[1024];
    ISO_CHECK(replace_once(complete, "\"arch\":\"test\"",
                           "\"arch\":\"test\",\"profile\":\"release\"",
                           profiled, sizeof(profiled)));
    ISO_CHECK(mb_replay_json(profiled, strlen(profiled), &report) == MB_OK);
    mb_report_free(&report);
  }
  {
    char whitespace_metadata[1024];
    char profiled_whitespace_metadata[1024];
    ISO_CHECK(replace_once(complete, "\"runtime\":\"C17\"",
                           "\"runtime\":\"\\u2003\"",
                           whitespace_metadata, sizeof(whitespace_metadata)));
    ISO_CHECK(replace_once(whitespace_metadata, "\"arch\":\"test\"",
                           "\"arch\":\"test\",\"profile\":\" \"",
                           profiled_whitespace_metadata,
                           sizeof(profiled_whitespace_metadata)));
    ISO_CHECK(mb_replay_json(profiled_whitespace_metadata,
                             strlen(profiled_whitespace_metadata),
                             &report) == MB_OK);
    mb_report_free(&report);
  }
  {
    char unicode_workload[1024];
    ISO_CHECK(replace_once(complete, "\"workload\":\"guest-tree-walk-8\"",
                           "\"workload\":\"guest-\\u03bb\"",
                           unicode_workload, sizeof(unicode_workload)));
    ISO_CHECK(mb_replay_json(unicode_workload, strlen(unicode_workload),
                             &report) == MB_OK);
    mb_report_free(&report);
  }
  {
    char unicode_whitespace[1024];
    ISO_CHECK(replace_once(complete, "\"workload\":\"guest-tree-walk-8\"",
                           "\"workload\":\"\\u2003\"",
                           unicode_whitespace, sizeof(unicode_whitespace)));
    ISO_CHECK(mb_replay_json(unicode_whitespace, strlen(unicode_whitespace),
                             &report) == MB_OK);
    mb_report_free(&report);
  }
  for (i = 0; i < sizeof(invalid) / sizeof(invalid[0]); i++) {
    char changed[2048];
    MBAllocator allocator;
    AllocationState allocation = {0, 0, 0, 0};
    MBReport rejected;
    ISO_CHECK(replace_once(invalid[i].source, invalid[i].needle,
                           invalid[i].replacement, changed, sizeof(changed)));
    allocator.allocate = test_allocate;
    allocator.release = test_release;
    allocator.context = &allocation;
    ISO_CHECK(reject_replay(changed));
    ISO_CHECK(mb_replay_json_with_allocator(changed, strlen(changed),
                                            &allocator, &rejected) == MB_JSON);
    ISO_CHECK(rejected.pairs == NULL && rejected.pair_count == 0);
    ISO_CHECK(allocation.allocations == allocation.releases);
  }
}
static void test_replay_argument_validation(void) {
  static const char valid[] =
      "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[],"
      "\"revision\":\"r\",\"runtime\":\"C17\",\"os\":\"test\","
      "\"arch\":\"test\",\"config\":{\"pairs\":1,"
      "\"confidence_percent\":95,\"minimum_factor\":1.05,\"seed\":0,"
      "\"target_ns\":1,\"max_batch\":1},\"batch_size\":1,"
      "\"pairs\":[{\"baseline_ns\":1,\"candidate_ns\":1,"
      "\"order\":\"baseline-first\"}],\"analysis\":{\"ratio\":1,"
      "\"interval\":null,\"verdict\":\"inconclusive\"}}";
  MBReport report;
  MBAllocator incomplete = {test_allocate, NULL, NULL};
  ISO_CHECK(mb_replay_json(NULL, 1, &report) == MB_ARGUMENT);
  ISO_CHECK(mb_replay_json(valid, sizeof(valid) - 1, NULL) == MB_ARGUMENT);
  ISO_CHECK(mb_replay_json_with_allocator(valid, sizeof(valid) - 1,
                                          &incomplete, &report) == MB_ARGUMENT);
  ISO_CHECK(reject_replay("{"));
  ISO_CHECK(mb_replay_json(NULL, 0, &report) == MB_JSON);
}
int main(void) {
  static MBPair oversized[10001];
  MBPair p[3];
  MBAnalysis a;
  MBConfig c = mb_default_config();
  MBReport r, r2;
  FakeClock clock = {0, 100, 0, 0, 0};
  Batch b = {7, 0}, d = {7, 0};
  int64_t x, y;
  size_t i, base = 0, cand = 0;
  test_fixture_cases();
  test_shared_fixture();
  test_replay_schema_mutations();
  test_portable_core_report_metadata();
  test_replay_argument_validation();
  test_generated();
  ISO_CHECK_STR_EQ(mb_status_name(MB_OK), "ok");
  ISO_CHECK_STR_EQ(mb_status_name(MB_ARGUMENT), "argument");
  ISO_CHECK_STR_EQ(mb_status_name(MB_LIMIT), "limit");
  ISO_CHECK_STR_EQ(mb_status_name(MB_MEMORY), "memory");
  ISO_CHECK_STR_EQ(mb_status_name(MB_CLOCK), "clock");
  ISO_CHECK_STR_EQ(mb_status_name(MB_WORKLOAD), "workload");
  ISO_CHECK_STR_EQ(mb_status_name(MB_CHECKSUM), "checksum");
  ISO_CHECK_STR_EQ(mb_status_name(MB_ANALYSIS), "analysis");
  ISO_CHECK_STR_EQ(mb_status_name(MB_JSON), "json");
  ISO_CHECK_STR_EQ(mb_status_name((MBStatus)99), "unknown");
  ISO_CHECK_STR_EQ(mb_verdict_name(MB_INCONCLUSIVE), "inconclusive");
  ISO_CHECK_STR_EQ(mb_verdict_name(MB_FASTER), "faster");
  ISO_CHECK_STR_EQ(mb_verdict_name(MB_SLOWER), "slower");
  ISO_CHECK_STR_EQ(mb_verdict_name(MB_EQUIVALENT), "equivalent");
  ISO_CHECK_STR_EQ(mb_verdict_name((MBVerdict)99), "inconclusive");
  mb_report_free(NULL);
  p[0] = (MBPair){100, 100, MB_BASELINE_FIRST};
  {
    AllocationState allocation = {1, 0, 0, 0};
    MBAllocator allocator = {test_allocate, test_release, &allocation};
    ISO_CHECK(mb_analyze_with_allocator(p, 1, 95, 1.05, &a, &allocator) ==
              MB_MEMORY);
    ISO_CHECK(a.verdict == MB_INCONCLUSIVE && a.ratio == 0.0);
  }
  ISO_CHECK(mb_analyze(NULL, 0, 95, 1.05, &a) == MB_ARGUMENT);
  ISO_CHECK(mb_analyze(p, 1, 95, 1.05, NULL) == MB_ARGUMENT);
  ISO_CHECK(mb_analyze(NULL, 1, 95, 1.05, &a) == MB_ARGUMENT);
  ISO_CHECK(mb_analyze(oversized, 10001, 95, 1.05, &a) == MB_LIMIT);
  ISO_CHECK(mb_analyze(p, 1, 0, 1.05, &a) == MB_ARGUMENT);
  ISO_CHECK(mb_analyze(p, 1, 100, 1.05, &a) == MB_ARGUMENT);
  ISO_CHECK(mb_analyze(p, 1, 95, 1.0, &a) == MB_ARGUMENT);
  ISO_CHECK(mb_analyze(p, 1, 95, NAN, &a) == MB_ARGUMENT);
  {
    AllocationState allocation = {0, 0, 0, 0};
    MBAllocator tracked = {test_allocate, test_release, &allocation};
    p[0] = (MBPair){100, 100, MB_BASELINE_FIRST};
    ISO_CHECK(mb_analyze_with_allocator(p, 1, 95, 1.05, &a, &tracked) ==
              MB_OK);
    ISO_CHECK(allocation.allocations == 1 && allocation.releases == 1);
    tracked.release = NULL;
    ISO_CHECK(mb_analyze_with_allocator(p, 1, 95, 1.05, &a, &tracked) ==
              MB_ARGUMENT);
  }
  p[0].baseline_ns = 1;
  p[0].candidate_ns = 0;
  p[0].order = MB_BASELINE_FIRST;
  ISO_CHECK(mb_analyze(p, 1, 95, 1.05, &a) == MB_ARGUMENT);
  p[0] = (MBPair){1, 1, (MBOrder)99};
  ISO_CHECK(mb_analyze(p, 1, 95, 1.05, &a) == MB_ARGUMENT);
  p[0] = (MBPair){100, 50, MB_BASELINE_FIRST};
  ISO_CHECK(mb_analyze(p, 1, 95, 1.05, &a) == MB_OK &&
            a.verdict == MB_INCONCLUSIVE);
  {
    MBPair slow[] = {{100, 200, MB_BASELINE_FIRST},
                     {100, 200, MB_CANDIDATE_FIRST},
                     {100, 200, MB_BASELINE_FIRST},
                     {100, 200, MB_CANDIDATE_FIRST},
                     {100, 200, MB_BASELINE_FIRST},
                     {100, 200, MB_CANDIDATE_FIRST}};
    ISO_CHECK(mb_analyze(slow, 6, 95, 1.05, &a) == MB_OK &&
              a.verdict == MB_SLOWER);
  }
  c.pairs = 3;
  c.target_ns = 50;
  c.max_batch = 8;
  c.seed = INT64_MIN;
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_OK);
  ISO_CHECK(r.pair_count == 3 && r.batch_size == 1);
  for (i = 0; i < r.pair_count; i++) {
    if (r.pairs[i].order == MB_BASELINE_FIRST)
      base++;
    else
      cand++;
  }
  ISO_CHECK(base == 2 && cand == 1);
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r2) ==
            MB_OK);
  for (i = 0; i < r.pair_count; i++)
    ISO_CHECK(r.pairs[i].order == r2.pairs[i].order);
  mb_report_free(&r2);
  mb_report_free(&r);
  ISO_CHECK(r.pairs == NULL && r.pair_count == 0);
  {
    AllocationState allocation = {1, 0, 0, 0};
    MBAllocator allocator = {test_allocate, test_release, &allocation};
    FakeClock alloc_clock = {0, 100, 0, 0, 0};
    ISO_CHECK(mb_collect_with_allocator(&c, 7, fake_clock, &alloc_clock, batch,
                                        &b, batch, &d, &allocator,
                                        &r) == MB_MEMORY);
    ISO_CHECK(r.pairs == NULL && allocation.allocations == 1 &&
              allocation.releases == 0);
  }
  {
    AllocationState allocation = {0, 0, 0, 0};
    MBAllocator allocator = {test_allocate, test_release, &allocation};
    FakeClock alloc_clock = {0, 100, 0, 0, 0};
    ISO_CHECK(mb_collect_with_allocator(&c, 7, fake_clock, &alloc_clock, batch,
                                        &b, batch, &d, &allocator,
                                        &r) == MB_OK);
    ISO_CHECK(allocation.allocations == 2 && allocation.releases == 1);
    mb_report_free(&r);
    ISO_CHECK(allocation.releases == 2);
    mb_report_free(&r);
    ISO_CHECK(allocation.releases == 2);
  }
  {
    AllocationState allocation = {0, 2, 0, 0};
    MBAllocator allocator = {test_allocate, test_release, &allocation};
    FakeClock alloc_clock = {0, 100, 0, 0, 0};
    ISO_CHECK(mb_collect_with_allocator(&c, 7, fake_clock, &alloc_clock, batch,
                                        &b, batch, &d, &allocator,
                                        &r) == MB_MEMORY);
    ISO_CHECK(r.pairs == NULL && allocation.allocations == 2 &&
              allocation.releases == 1);
  }
  c.max_batch = INT64_MAX;
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_OK);
  mb_report_free(&r);
  c.max_batch = 8;
  b.each = -7;
  d.each = -7;
  ISO_CHECK(mb_collect(&c, -7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_OK);
  mb_report_free(&r);
  b.each = 7;
  d.each = 7;
  ISO_CHECK(mb_indexed_array(0, 1, &x) == MB_OK);
  ISO_CHECK(mb_indexed_array(1, 1, &y) == MB_OK);
  ISO_CHECK(x == y && x > 0);
  ISO_CHECK(mb_indexed_array(0, 0, &x) == MB_OK && x == 0);
  {
    const char *json =
        "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[\"bench\"]"
        ",\"revision\":\"r\",\"runtime\":\"C17\",\"os\":\"test\",\"arch\":"
        "\"test\",\"config\":{\"pairs\":1,\"confidence_percent\":95,\"minimum_"
        "factor\":1.05,\"seed\":-9223372036854775808,\"target_ns\":1,\"max_"
        "batch\":9223372036854775807},\"batch_size\":1,\"pairs\":[{\"baseline_"
        "ns\":9223372036854775807,\"candidate_ns\":9223372036854775807,"
        "\"order\":\"baseline-first\"}],\"analysis\":{\"ratio\":1,\"interval\":"
        "null,\"verdict\":\"inconclusive\"}}";
    ISO_CHECK(mb_replay_json(json, strlen(json), &r) == MB_OK);
    mb_report_free(&r);
    {
      AllocationState allocation = {1, 0, 0, 0};
      MBAllocator allocator = {test_allocate, test_release, &allocation};
      ISO_CHECK(mb_replay_json_with_allocator(json, strlen(json), &allocator,
                                              &r) == MB_MEMORY);
      ISO_CHECK(r.pairs == NULL && allocation.allocations == 1 &&
                allocation.releases == 0);
    }
    {
      AllocationState allocation = {0, 0, 0, 0};
      MBAllocator allocator = {test_allocate, test_release, &allocation};
      ISO_CHECK(mb_replay_json_with_allocator(json, strlen(json), &allocator,
                                              &r) == MB_OK);
      mb_report_free(&r);
      ISO_CHECK(allocation.allocations == 1 && allocation.releases == 1);
    }
  }
  {
    const char *json =
        "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[\"b\"],"
        "\"revision\":\"r\",\"runtime\":\"C17\",\"os\":\"test\",\"arch\":"
        "\"test\",\"config\":{\"pairs\":1,\"confidence_percent\":95,\"minimum_"
        "factor\":1.05,\"seed\":0,\"target_ns\":1,\"max_batch\":1},\"batch_"
        "size\":1,\"pairs\":[{\"baseline_ns\":1,\"candidate_ns\":1,\"order\":"
        "\"candidate-first\"}],\"analysis\":{\"ratio\":1,\"interval\":null,"
        "\"verdict\":\"inconclusive\"}}";
    ISO_CHECK(mb_replay_json(json, strlen(json), &r) == MB_JSON);
  }
  clock.fail = 1;
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_CLOCK);
  ISO_CHECK(r.pairs == NULL);
  {
    FakeClock regressing = {0, 100, 0, 0, 3};
    ISO_CHECK(mb_collect(&c, 7, fake_clock, &regressing, batch, &b, batch, &d,
                         &r) == MB_CLOCK);
  }
  {
    FakeClock negative_tick = {INT64_MIN, 0, 0, 0, 0};
    ISO_CHECK(mb_collect(&c, 7, fake_clock, &negative_tick, batch, &b, batch,
                         &d, &r) == MB_CLOCK);
  }
  {
    FakeClock zero = {0, 0, 0, 0, 0};
    c.max_batch = 2;
    ISO_CHECK(mb_collect(&c, 7, fake_clock, &zero, batch, &b, batch, &d, &r) ==
              MB_LIMIT);
  }
  {
    FakeClock too_slow = {0, 100, 0, 0, 0};
    c.max_batch = 1;
    c.target_ns = 1000;
    ISO_CHECK(mb_collect(&c, 7, fake_clock, &too_slow, batch, &b, batch, &d,
                         &r) == MB_LIMIT);
  }
  {
    FakeClock overflow = {0, 100, 0, 0, 0};
    c.target_ns = 150;
    c.max_batch = 2;
    b.each = INT64_MAX;
    d.each = INT64_MAX;
    ISO_CHECK(mb_collect(&c, INT64_MAX, fake_clock, &overflow, batch, &b, batch,
                         &d, &r) == MB_LIMIT);
    b.each = 7;
    d.each = 7;
  }
  d.each = 8;
  clock.fail = 0;
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_CHECKSUM);
  ISO_CHECK(r.pairs == NULL);
  d.each = 7;
  b.fail = 1;
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_WORKLOAD);
  b.fail = 0;
  c.max_batch = 0;
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_ARGUMENT);
  c = mb_default_config();
  ISO_CHECK(mb_collect(NULL, 7, fake_clock, &clock, batch, &b, batch, &d, &r) ==
            MB_ARGUMENT);
  ISO_CHECK(mb_collect(&c, 7, NULL, &clock, batch, &b, batch, &d, &r) ==
            MB_ARGUMENT);
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, NULL, &b, batch, &d, &r) ==
            MB_ARGUMENT);
  ISO_CHECK(mb_collect(&c, 7, fake_clock, &clock, batch, &b, NULL, &d, &r) ==
            MB_ARGUMENT);
  c.pairs = 1;
  c.target_ns = 50;
  c.max_batch = 1;
  {
    FakeClock pair_clock = {0, 100, 0, 0, 0};
    SequencedBatch pair_batch = {7, 0, 3, 0};
    ISO_CHECK(mb_collect(&c, 7, fake_clock, &pair_clock, sequenced_batch,
                         &pair_batch, batch, &d, &r) == MB_WORKLOAD);
    ISO_CHECK(r.pairs == NULL && pair_batch.calls == 3);
  }
  {
    FakeClock pair_clock = {0, 100, 0, 0, 0};
    SequencedBatch pair_batch = {7, 0, 0, 3};
    ISO_CHECK(mb_collect(&c, 7, fake_clock, &pair_clock, sequenced_batch,
                         &pair_batch, batch, &d, &r) == MB_CHECKSUM);
    ISO_CHECK(r.pairs == NULL && pair_batch.calls == 3);
  }
  {
    const int64_t ticks[] = {0, 100, 100, 200, 200, 200};
    TickSequence pair_clock = {ticks, sizeof(ticks) / sizeof(ticks[0]), 0};
    ISO_CHECK(mb_collect(&c, 7, sequence_clock, &pair_clock, batch, &b, batch,
                         &d, &r) == MB_CLOCK);
    ISO_CHECK(r.pairs == NULL && pair_clock.index == 6);
  }
  c = mb_default_config();
  c.pairs = 1;
  ISO_CHECK(mb_indexed_array(0, 0, NULL) == MB_ARGUMENT);
  ISO_CHECK(mb_indexed_array(0, (size_t)INT64_MAX, &x) == MB_LIMIT);
  return ISO_TEST_RESULT();
}
