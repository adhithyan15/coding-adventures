#include "microbenchmark.h"
#include "rank_statistics.h"
#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
const char *mb_status_name(MBStatus s) {
  switch (s) {
  case MB_OK:
    return "ok";
  case MB_ARGUMENT:
    return "argument";
  case MB_LIMIT:
    return "limit";
  case MB_MEMORY:
    return "memory";
  case MB_CLOCK:
    return "clock";
  case MB_WORKLOAD:
    return "workload";
  case MB_CHECKSUM:
    return "checksum";
  case MB_ANALYSIS:
    return "analysis";
  case MB_JSON:
    return "json";
  default:
    return "unknown";
  }
}
const char *mb_verdict_name(MBVerdict v) {
  switch (v) {
  case MB_FASTER:
    return "faster";
  case MB_SLOWER:
    return "slower";
  case MB_EQUIVALENT:
    return "equivalent";
  default:
    return "inconclusive";
  }
}
static int valid_allocator(const MBAllocator *a) {
  return !a || (a->allocate && a->release);
}
static void *allocate_with(const MBAllocator *a, size_t n) {
  return a ? a->allocate(n, a->context) : malloc(n);
}
static void release_with(const MBAllocator *a, void *p) {
  if (a)
    a->release(p, a->context);
  else
    free(p);
}
MBConfig mb_default_config(void) {
  MBConfig c;
  c.pairs = 40;
  c.confidence_percent = 95;
  c.minimum_factor = 1.05;
  c.seed = 0;
  c.target_ns = 100000;
  c.max_batch = 65536;
  return c;
}
static int valid_config(const MBConfig *c) {
  return c && c->pairs > 0 && c->pairs <= 10000 && c->confidence_percent >= 1 &&
         c->confidence_percent <= 99 && isfinite(c->minimum_factor) &&
         c->minimum_factor > 1.0 && c->target_ns > 0 && c->max_batch > 0;
}
MBStatus mb_analyze_with_allocator(const MBPair *pairs, size_t count,
                                   unsigned confidence, double factor,
                                   MBAnalysis *out,
                                   const MBAllocator *allocator) {
  double *ratios;
  size_t i;
  RSInterval ci;
  RSStatus rs;
  double median, lower, upper;
  if (!out)
    return MB_ARGUMENT;
  memset(out, 0, sizeof(*out));
  out->verdict = MB_INCONCLUSIVE;
  if ((!pairs && count) || !valid_allocator(allocator))
    return MB_ARGUMENT;
  if (!count)
    return MB_ARGUMENT;
  if (count > 10000 || count > SIZE_MAX / sizeof(double))
    return MB_LIMIT;
  if (confidence < 1 || confidence > 99 || !isfinite(factor) || factor <= 1.0)
    return MB_ARGUMENT;
  ratios = (double *)allocate_with(allocator, count * sizeof(double));
  if (!ratios)
    return MB_MEMORY;
  for (i = 0; i < count; i++) {
    if (pairs[i].baseline_ns <= 0 || pairs[i].candidate_ns <= 0 ||
        (pairs[i].order != MB_BASELINE_FIRST &&
         pairs[i].order != MB_CANDIDATE_FIRST)) {
      release_with(allocator, ratios);
      return MB_ARGUMENT;
    }
    ratios[i] = (double)pairs[i].candidate_ns / (double)pairs[i].baseline_ns;
  }
  rs = rs_median(ratios, count, &median);
  if (rs != RS_OK) {
    release_with(allocator, ratios);
    return MB_ANALYSIS;
  }
  rs = rs_median_interval(ratios, count, confidence, &ci);
  release_with(allocator, ratios);
  if (rs != RS_OK)
    return MB_ANALYSIS;
  out->ratio = median;
  out->interval_available = ci.available;
  out->low = ci.low;
  out->high = ci.high;
  if (ci.available) {
    lower = 1.0 / factor;
    upper = factor;
    if (ci.high < lower)
      out->verdict = MB_FASTER;
    else if (ci.low > upper)
      out->verdict = MB_SLOWER;
    else if (ci.low >= lower && ci.high <= upper)
      out->verdict = MB_EQUIVALENT;
  }
  return MB_OK;
}
MBStatus mb_analyze(const MBPair *pairs, size_t count, unsigned confidence,
                    double factor, MBAnalysis *out) {
  return mb_analyze_with_allocator(pairs, count, confidence, factor, out, NULL);
}
void mb_report_free(MBReport *r) {
  if (!r)
    return;
  if (r->pairs) {
    if (r->release_pairs)
      r->release_pairs(r->pairs, r->allocator_context);
    else
      free(r->pairs);
  }
  memset(r, 0, sizeof(*r));
}
static uint64_t next_random(uint64_t *s) {
  uint64_t x = *s;
  if (!x)
    x = UINT64_C(0x9e3779b97f4a7c15);
  x ^= x << 13;
  x ^= x >> 7;
  x ^= x << 17;
  *s = x;
  return x;
}
static int read_tick(MBClock clock, void *context, int has_last, int64_t *last,
                     int64_t *tick) {
  if (!clock(context, tick) || *tick < 0 || (has_last && *tick < *last))
    return 0;
  *last = *tick;
  return 1;
}
static int run_batch(MBClock clock, void *cc, MBBatch batch, void *bc, size_t n,
                     int64_t expected, int64_t *elapsed, int *has_last,
                     int64_t *last) {
  int64_t a, b, got;
  if (!read_tick(clock, cc, *has_last, last, &a))
    return 0;
  *has_last = 1;
  if (!batch(bc, n, &got))
    return -1;
  if (!read_tick(clock, cc, *has_last, last, &b))
    return 0;
  if (got != expected)
    return -2;
  *elapsed = b - a;
  return 1;
}
static int expected_for(int64_t each, size_t n, int64_t *out) {
  uint64_t k, mag, limit;
  if ((uintmax_t)n > (uintmax_t)INT64_MAX)
    return 0;
  k = (uint64_t)n;
  if (each > 0) {
    if (k > (uint64_t)INT64_MAX / (uint64_t)each)
      return 0;
  } else if (each < 0) {
    mag = (uint64_t)(-(each + 1)) + 1u;
    limit = (UINT64_C(1) << 63) / mag;
    if (k > limit)
      return 0;
  }
  *out = each * (int64_t)k;
  return 1;
}
MBStatus mb_collect_with_allocator(const MBConfig *c, int64_t expected,
                                   MBClock clock, void *cc, MBBatch base,
                                   void *bc, MBBatch cand, void *dc,
                                   const MBAllocator *allocator,
                                   MBReport *out) {
  size_t size = 1, i, base_first;
  size_t max_size;
  int64_t want, db, dcand, last = 0;
  int a, b, has_last = 0;
  uint64_t rng;
  MBStatus status = MB_OK;
  if (!out)
    return MB_ARGUMENT;
  memset(out, 0, sizeof(*out));
  if (!valid_config(c) || !clock || !base || !cand ||
      !valid_allocator(allocator))
    return MB_ARGUMENT;
  max_size = (uintmax_t)c->max_batch > (uintmax_t)SIZE_MAX
                 ? SIZE_MAX
                 : (size_t)c->max_batch;
  if (!expected_for(expected, 1, &want))
    return MB_LIMIT;
  {
    int64_t check;
    if (!base(bc, 1, &check))
      return MB_WORKLOAD;
    if (check != expected)
      return MB_CHECKSUM;
    if (!cand(dc, 1, &check))
      return MB_WORKLOAD;
    if (check != expected)
      return MB_CHECKSUM;
  }
  for (;;) {
    if (!expected_for(expected, size, &want))
      return MB_LIMIT;
    a = run_batch(clock, cc, base, bc, size, want, &db, &has_last, &last);
    if (a < 0)
      return a == -1 ? MB_WORKLOAD : MB_CHECKSUM;
    if (!a)
      return MB_CLOCK;
    b = run_batch(clock, cc, cand, dc, size, want, &dcand, &has_last, &last);
    if (b < 0)
      return b == -1 ? MB_WORKLOAD : MB_CHECKSUM;
    if (!b)
      return MB_CLOCK;
    if (db >= c->target_ns && dcand >= c->target_ns)
      break;
    if (size >= max_size)
      return MB_LIMIT;
    if (size > max_size / 2)
      size = max_size;
    else
      size *= 2;
  }
  if (c->pairs > SIZE_MAX / sizeof(MBPair))
    return MB_LIMIT;
  out->pairs = (MBPair *)allocate_with(allocator, c->pairs * sizeof(MBPair));
  if (!out->pairs)
    return MB_MEMORY;
  memset(out->pairs, 0, c->pairs * sizeof(MBPair));
  if (allocator) {
    out->release_pairs = allocator->release;
    out->allocator_context = allocator->context;
  }
  out->pair_count = c->pairs;
  out->config = *c;
  out->batch_size = size;
  base_first = (c->pairs + 1) / 2;
  for (i = 0; i < c->pairs; i++)
    out->pairs[i].order =
        i < base_first ? MB_BASELINE_FIRST : MB_CANDIDATE_FIRST;
  rng = (uint64_t)c->seed;
  for (i = c->pairs; i > 1; i--) {
    size_t j = (size_t)(next_random(&rng) % (uint64_t)i);
    MBOrder t = out->pairs[i - 1].order;
    out->pairs[i - 1].order = out->pairs[j].order;
    out->pairs[j].order = t;
  }
  for (i = 0; i < c->pairs; i++) {
    if (out->pairs[i].order == MB_BASELINE_FIRST) {
      a = run_batch(clock, cc, base, bc, size, want, &db, &has_last, &last);
      b = a > 0 ? run_batch(clock, cc, cand, dc, size, want, &dcand, &has_last,
                            &last)
                : a;
    } else {
      b = run_batch(clock, cc, cand, dc, size, want, &dcand, &has_last, &last);
      a = b > 0 ? run_batch(clock, cc, base, bc, size, want, &db, &has_last,
                            &last)
                : b;
    }
    if (a <= 0 || b <= 0) {
      status = a == -2 || b == -2   ? MB_CHECKSUM
               : a == -1 || b == -1 ? MB_WORKLOAD
                                    : MB_CLOCK;
      goto fail;
    }
    if (db <= 0 || dcand <= 0) {
      status = MB_CLOCK;
      goto fail;
    }
    out->pairs[i].baseline_ns = db;
    out->pairs[i].candidate_ns = dcand;
  }
  status = mb_analyze_with_allocator(out->pairs, out->pair_count,
                                     c->confidence_percent, c->minimum_factor,
                                     &out->analysis, allocator);
  if (status != MB_OK)
    goto fail;
  return MB_OK;
fail:
  mb_report_free(out);
  return status;
}
MBStatus mb_collect(const MBConfig *c, int64_t expected, MBClock clock,
                    void *cc, MBBatch base, void *bc, MBBatch cand, void *dc,
                    MBReport *out) {
  return mb_collect_with_allocator(c, expected, clock, cc, base, bc, cand, dc,
                                   NULL, out);
}
MBStatus mb_indexed_array(int iteration, size_t operations, int64_t *checksum) {
  uint64_t one = 0, total = 0;
  size_t i, j;
  uint32_t values[4096];
  if (!checksum)
    return MB_ARGUMENT;
  for (i = 0; i < 4096; i++) {
    values[i] = (uint32_t)((17u * (uint64_t)i + 3u) % 251u);
    one += values[i];
  }
  if ((uintmax_t)operations > (uintmax_t)INT64_MAX)
    return MB_LIMIT;
  if (operations && one > (uint64_t)INT64_MAX / operations)
    return MB_LIMIT;
  for (j = 0; j < operations; j++) {
    uint64_t sum = 0;
    if (iteration) {
      const uint32_t *p = values;
      for (i = 0; i < 4096; i++)
        sum += p[i];
    } else
      for (i = 0; i < 4096; i++)
        sum += values[i];
    total += sum;
  }
  *checksum = (int64_t)total;
  return MB_OK;
}
