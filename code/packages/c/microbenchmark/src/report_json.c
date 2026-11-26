#include "bounded_json.h"
#include "microbenchmark.h"
#include <math.h>
#include <stdlib.h>
#include <string.h>
static const BJsonValue *member(const BJsonValue *v, const char *wanted) {
  size_t i, n;
  const char *name;
  size_t len;
  const BJsonValue *child, *found = NULL;
  n = bj_size(v);
  for (i = 0; i < n; i++) {
    if (!bj_member(v, i, &name, &len, &child))
      return NULL;
    if (len == strlen(wanted) && memcmp(name, wanted, len) == 0) {
      if (found)
        return NULL;
      found = child;
    }
  }
  return found;
}
static int get_i64(const BJsonValue *v, int64_t *out) {
  return bj_kind(v) == BJ_INTEGER && bj_integer(v, out);
}
static int get_double(const BJsonValue *v, double *out) {
  return (bj_kind(v) == BJ_FLOAT || bj_kind(v) == BJ_INTEGER) &&
         bj_number(v, out) && isfinite(*out);
}
static int get_string(const BJsonValue *v, const char **s, size_t *n) {
  return bj_string(v, s, n);
}
static uint32_t next_codepoint(const unsigned char **cursor) {
  const unsigned char *s = *cursor;
  uint32_t c = *s++;
  if (c >= 0xc2u && c <= 0xdfu) {
    c = ((c & 0x1fu) << 6) | (*s++ & 0x3fu);
  } else if (c >= 0xe0u && c <= 0xefu) {
    c = ((c & 0x0fu) << 12) | ((*s++ & 0x3fu) << 6);
    c |= *s++ & 0x3fu;
  } else if (c >= 0xf0u && c <= 0xf4u) {
    c = ((c & 0x07u) << 18) | ((*s++ & 0x3fu) << 12);
    c |= (*s++ & 0x3fu) << 6;
    c |= *s++ & 0x3fu;
  }
  *cursor = s;
  return c;
}
static int unicode_whitespace(uint32_t c) {
  return (c >= 0x09u && c <= 0x0du) || c == 0x20u || c == 0x85u ||
         c == 0xa0u || c == 0x1680u || (c >= 0x2000u && c <= 0x200au) ||
         c == 0x2028u || c == 0x2029u || c == 0x202fu || c == 0x205fu ||
         c == 0x3000u;
}
static int text_valid(const char *s, size_t n, int require_nonblank) {
  const unsigned char *cursor = (const unsigned char *)s;
  const unsigned char *end = cursor + n;
  int nonblank = 0;
  if (memchr(s, '\0', n))
    return 0;
  while (cursor < end) {
    uint32_t c = next_codepoint(&cursor);
    if (!unicode_whitespace(c))
      nonblank = 1;
  }
  return !require_nonblank || nonblank;
}
static int workload_nonblank_ascii(const char *s, size_t n) {
  size_t i;
  for (i = 0; i < n; i++) {
    unsigned char c = (unsigned char)s[i];
    if (c != 0x20u && (c < 0x09u || c > 0x0du))
      return 1;
  }
  return 0;
}
static int allowed(const char *s, size_t n) {
  static const char *keys[] = {
      "workload", "invocation", "revision",   "runtime", "os",      "arch",
      "profile",  "config",     "batch_size", "pairs",   "analysis"};
  size_t i;
  for (i = 0; i < sizeof(keys) / sizeof(keys[0]); i++)
    if (strlen(keys[i]) == n && memcmp(s, keys[i], n) == 0)
      return 1;
  return 0;
}
static int replay_allocator_valid(const MBAllocator *a) {
  return !a || (a->allocate && a->release);
}
static void *replay_allocate(const MBAllocator *a, size_t n) {
  return a ? a->allocate(n, a->context) : malloc(n);
}
MBStatus mb_replay_json_with_allocator(const char *json, size_t length,
                                       const MBAllocator *allocator,
                                       MBReport *out) {
  BJsonDoc *doc = NULL;
  BJsonError error;
  const BJsonValue *root, *v, *cfg, *arr, *analysis, *interval;
  size_t i, n, base_first = 0, cand_first = 0;
  int64_t x;
  double d;
  const char *s;
  size_t sl;
  MBStatus status = MB_JSON;
  if (!out || (!json && length))
    return MB_ARGUMENT;
  memset(out, 0, sizeof(*out));
  if (!replay_allocator_valid(allocator))
    return MB_ARGUMENT;
  if (bj_parse(json, length, &doc, &error) != BJ_OK)
    return MB_JSON;
  root = bj_root(doc);
  if (bj_kind(root) != BJ_OBJECT)
    goto done;
  n = bj_size(root);
  if (n < 4 || n > 11)
    goto done;
  for (i = 0; i < n; i++) {
    const char *name;
    size_t nl, j;
    const BJsonValue *child;
    if (!bj_member(root, i, &name, &nl, &child) || !allowed(name, nl))
      goto done;
    for (j = 0; j < i; j++) {
      const char *prior;
      size_t pl;
      const BJsonValue *ignored;
      if (!bj_member(root, j, &prior, &pl, &ignored))
        goto done;
      if (pl == nl && memcmp(prior, name, nl) == 0)
        goto done;
    }
  }
  {
    const char *required[] = {"config", "batch_size", "pairs", "analysis"};
    const char *metadata[] = {"workload", "invocation", "revision",
                              "runtime",  "os",         "arch"};
    size_t metadata_present = 0;
    for (i = 0; i < sizeof(required) / sizeof(required[0]); i++)
      if (!member(root, required[i]))
        goto done;
    for (i = 0; i < sizeof(metadata) / sizeof(metadata[0]); i++)
      if (member(root, metadata[i]))
        metadata_present++;
    if (metadata_present != 0 && metadata_present != 6)
      goto done;
    v = member(root, "profile");
    if (v && metadata_present != 6)
      goto done;
    if (metadata_present == 6) {
      const char *strings[] = {"workload", "revision", "runtime", "os",
                               "arch",     "profile"};
      for (i = 0; i < sizeof(strings) / sizeof(strings[0]); i++) {
        v = member(root, strings[i]);
        if (v) {
          if (!get_string(v, &s, &sl) || !text_valid(s, sl, 0))
            goto done;
          if (i == 0 ? !workload_nonblank_ascii(s, sl)
                     : !text_valid(s, sl, 1))
            goto done;
        } else if (i < 5)
          goto done;
      }
      v = member(root, "invocation");
      if (bj_kind(v) != BJ_ARRAY)
        goto done;
      for (i = 0; i < bj_size(v); i++)
        if (!get_string(bj_at(v, i), &s, &sl) || !text_valid(s, sl, 0))
          goto done;
    }
  }
  cfg = member(root, "config");
  if (bj_kind(cfg) != BJ_OBJECT || bj_size(cfg) != 6)
    goto done;
  v = member(cfg, "pairs");
  if (!get_i64(v, &x) || x < 1 || x > 10000)
    goto done;
  out->config.pairs = (size_t)x;
  v = member(cfg, "confidence_percent");
  if (!get_i64(v, &x) || x < 1 || x > 99)
    goto done;
  out->config.confidence_percent = (unsigned)x;
  v = member(cfg, "minimum_factor");
  if (!get_double(v, &d) || d <= 1.0)
    goto done;
  out->config.minimum_factor = d;
  v = member(cfg, "seed");
  if (!get_i64(v, &out->config.seed))
    goto done;
  v = member(cfg, "target_ns");
  if (!get_i64(v, &out->config.target_ns) || out->config.target_ns <= 0)
    goto done;
  v = member(cfg, "max_batch");
  if (!get_i64(v, &out->config.max_batch) || out->config.max_batch <= 0)
    goto done;
  v = member(root, "batch_size");
  if (!get_i64(v, &x) || x < 1 || (uintmax_t)x > (uintmax_t)SIZE_MAX ||
      x > out->config.max_batch)
    goto done;
  out->batch_size = (size_t)x;
  arr = member(root, "pairs");
  if (bj_kind(arr) != BJ_ARRAY || bj_size(arr) != out->config.pairs)
    goto done;
  if (out->config.pairs > SIZE_MAX / sizeof(MBPair)) {
    status = MB_LIMIT;
    goto done;
  }
  out->pairs =
      (MBPair *)replay_allocate(allocator, out->config.pairs * sizeof(MBPair));
  if (!out->pairs) {
    status = MB_MEMORY;
    goto done;
  }
  memset(out->pairs, 0, out->config.pairs * sizeof(MBPair));
  if (allocator) {
    out->release_pairs = allocator->release;
    out->allocator_context = allocator->context;
  }
  out->pair_count = out->config.pairs;
  for (i = 0; i < out->pair_count; i++) {
    const BJsonValue *p = bj_at(arr, i);
    if (bj_kind(p) != BJ_OBJECT || bj_size(p) != 3)
      goto done;
    v = member(p, "baseline_ns");
    if (!get_i64(v, &out->pairs[i].baseline_ns) ||
        out->pairs[i].baseline_ns <= 0)
      goto done;
    v = member(p, "candidate_ns");
    if (!get_i64(v, &out->pairs[i].candidate_ns) ||
        out->pairs[i].candidate_ns <= 0)
      goto done;
    v = member(p, "order");
    if (!get_string(v, &s, &sl))
      goto done;
    if (sl == 14 && memcmp(s, "baseline-first", 14) == 0) {
      out->pairs[i].order = MB_BASELINE_FIRST;
      base_first++;
    } else if (sl == 15 && memcmp(s, "candidate-first", 15) == 0) {
      out->pairs[i].order = MB_CANDIDATE_FIRST;
      cand_first++;
    } else
      goto done;
  }
  if (base_first != (out->pair_count + 1) / 2 ||
      cand_first != out->pair_count / 2)
    goto done;
  status =
      mb_analyze(out->pairs, out->pair_count, out->config.confidence_percent,
                 out->config.minimum_factor, &out->analysis);
  if (status != MB_OK) {
    status = MB_JSON;
    goto done;
  }
  /* From this point every rejection is a schema/report mismatch rather than
     an allocation or analysis failure. Keep the default failure status until
     the entire serialized analysis has been checked. */
  status = MB_JSON;
  analysis = member(root, "analysis");
  if (bj_kind(analysis) != BJ_OBJECT || bj_size(analysis) != 3)
    goto done;
  v = member(analysis, "ratio");
  if (!get_double(v, &d) || d != out->analysis.ratio)
    goto done;
  interval = member(analysis, "interval");
  if (out->analysis.interval_available) {
    if (bj_kind(interval) != BJ_ARRAY || bj_size(interval) != 2 ||
        !get_double(bj_at(interval, 0), &d) || d != out->analysis.low ||
        !get_double(bj_at(interval, 1), &d) || d != out->analysis.high)
      goto done;
  } else if (bj_kind(interval) != BJ_NULL)
    goto done;
  v = member(analysis, "verdict");
  if (!get_string(v, &s, &sl) ||
      sl != strlen(mb_verdict_name(out->analysis.verdict)) ||
      memcmp(s, mb_verdict_name(out->analysis.verdict), sl) != 0)
    goto done;
  status = MB_OK;
done:
  bj_doc_free(&doc);
  if (status != MB_OK)
    mb_report_free(out);
  return status;
}
MBStatus mb_replay_json(const char *json, size_t length, MBReport *out) {
  return mb_replay_json_with_allocator(json, length, NULL, out);
}
