#if defined(_MSC_VER)
/* Keep the ISO stdio APIs; do not replace them with Microsoft-only *_s calls.
 */
#define _CRT_SECURE_NO_WARNINGS
#endif
#include "microbenchmark.h"
#include <errno.h>
#include <inttypes.h>
#include <limits.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if defined(_WIN32)
#define MB_OS "windows"
#elif defined(__APPLE__)
#define MB_OS "macos"
#elif defined(__linux__)
#define MB_OS "linux"
#else
#define MB_OS "unknown"
#endif
#if defined(_M_X64) || defined(__x86_64__)
#define MB_ARCH "x86_64"
#elif defined(_M_ARM64) || defined(__aarch64__)
#define MB_ARCH "aarch64"
#else
#define MB_ARCH "unknown"
#endif
#if defined(__clang__)
#define MB_RUNTIME "Clang-C17/" __clang_version__
#elif defined(_MSC_VER)
#define MB_STRINGIFY_INNER(x) #x
#define MB_STRINGIFY(x) MB_STRINGIFY_INNER(x)
#define MB_RUNTIME "MSVC-C17/" MB_STRINGIFY(_MSC_VER)
#else
#define MB_RUNTIME __VERSION__
#endif
typedef struct {
  uint32_t values[4096];
  uint64_t one_sum;
  int candidate;
} Work;
static void prepare(Work *w, int candidate) {
  size_t i;
  w->candidate = candidate;
  w->one_sum = 0;
  for (i = 0; i < 4096; i++)
    w->one_sum += w->values[i] = (uint32_t)((17u * (uint64_t)i + 3u) % 251u);
}
static int work_batch(void *context, size_t operations, int64_t *checksum) {
  Work *w = (Work *)context;
  uint64_t total = 0;
  size_t i, j;
  /* Compare in uintmax_t so a 32-bit size_t never narrows the quotient. */
  if ((uintmax_t)operations >
      (uintmax_t)((uint64_t)INT64_MAX / w->one_sum))
    return 0;
  for (j = 0; j < operations; j++) {
    uint64_t sum = 0;
    if (w->candidate) {
      const uint32_t *p = w->values;
      for (i = 0; i < 4096; i++)
        sum += p[i];
    } else
      for (i = 0; i < 4096; i++)
        sum += w->values[i];
    total += sum;
  }
  *checksum = (int64_t)total;
  return 1;
}
typedef struct {
  char *p;
  size_t n, cap;
  int failed;
} Buffer;
static void append(Buffer *b, const char *s) {
  size_t n = strlen(s);
  if (b->failed || n > SIZE_MAX - b->n - 1) {
    b->failed = 1;
    return;
  }
  if (b->n + n + 1 > b->cap) {
    size_t cap = b->cap ? b->cap : 1024;
    char *p;
    while (cap < b->n + n + 1) {
      if (cap > SIZE_MAX / 2) {
        b->failed = 1;
        return;
      }
      cap *= 2;
    }
    p = (char *)realloc(b->p, cap);
    if (!p) {
      b->failed = 1;
      return;
    }
    b->p = p;
    b->cap = cap;
  }
  memcpy(b->p + b->n, s, n + 1);
  b->n += n;
}
static void fmt(Buffer *b, const char *format, ...) {
  va_list ap;
  int n;
  char stack[256];
  va_start(ap, format);
  n = vsnprintf(stack, sizeof(stack), format, ap);
  va_end(ap);
  if (n < 0) {
    b->failed = 1;
    return;
  }
  if ((size_t)n < sizeof(stack)) {
    append(b, stack);
    return;
  }
  if ((size_t)n > SIZE_MAX - 1) {
    b->failed = 1;
    return;
  }
  {
    char *p = (char *)malloc((size_t)n + 1);
    if (!p) {
      b->failed = 1;
      return;
    }
    va_start(ap, format);
    if (vsnprintf(p, (size_t)n + 1, format, ap) != n)
      b->failed = 1;
    va_end(ap);
    if (!b->failed)
      append(b, p);
    free(p);
  }
}
static void quote(Buffer *b, const char *s) {
  const unsigned char *p = (const unsigned char *)s;
  append(b, "\"");
  while (*p) {
    char esc[7];
    if (*p == '"' || *p == '\\') {
      esc[0] = '\\';
      esc[1] = (char)*p;
      esc[2] = 0;
      append(b, esc);
    } else if (*p < 0x20) {
      (void)snprintf(esc, sizeof(esc), "\\u%04x", (unsigned)*p);
      append(b, esc);
    } else {
      esc[0] = (char)*p;
      esc[1] = 0;
      append(b, esc);
    }
    p++;
  }
  append(b, "\"");
}
static char *report_json(const MBReport *r, const char *revision, int argc,
                         char **argv) {
  Buffer b = {0};
  size_t i;
  append(&b, "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[");
  for (i = 0; i < (size_t)argc; i++) {
    if (i)
      append(&b, ",");
    quote(&b, argv[i]);
  }
  append(&b, "],\"revision\":");
  quote(&b, revision);
  append(&b, ",\"runtime\":");
  quote(&b, MB_RUNTIME);
  append(&b, ",\"os\":\"");
  append(&b, MB_OS);
  append(&b, "\",\"arch\":\"");
  append(&b, MB_ARCH);
  append(&b, "\",\"config\":{");
  fmt(&b,
      "\"pairs\":%llu,\"confidence_percent\":%u,\"minimum_factor\":%.17g,"
      "\"seed\":%" PRId64 ",\"target_ns\":%" PRId64 ",\"max_batch\":%" PRId64
      "},\"batch_size\":%llu,\"pairs\":[",
      (unsigned long long)r->config.pairs, r->config.confidence_percent,
      r->config.minimum_factor, r->config.seed, r->config.target_ns,
      r->config.max_batch, (unsigned long long)r->batch_size);
  for (i = 0; i < r->pair_count; i++) {
    if (i)
      append(&b, ",");
    fmt(&b,
        "{\"baseline_ns\":%" PRId64 ",\"candidate_ns\":%" PRId64 ",\"order\":",
        r->pairs[i].baseline_ns, r->pairs[i].candidate_ns);
    quote(&b, r->pairs[i].order == MB_BASELINE_FIRST ? "baseline-first"
                                                     : "candidate-first");
    append(&b, "}");
  }
  append(&b, "],\"analysis\":{");
  fmt(&b, "\"ratio\":%.17g,\"interval\":", r->analysis.ratio);
  if (r->analysis.interval_available)
    fmt(&b, "[%.17g,%.17g]", r->analysis.low, r->analysis.high);
  else
    append(&b, "null");
  append(&b, ",\"verdict\":");
  quote(&b, mb_verdict_name(r->analysis.verdict));
  append(&b, "}}\n");
  if (b.failed) {
    free(b.p);
    return NULL;
  }
  return b.p;
}
static int read_replay(const char *path) {
  FILE *f;
  long size;
  char *data;
  MBReport r;
  MBStatus s;
  f = fopen(path, "rb");
  if (!f) {
    fprintf(stderr, "cannot open replay input\n");
    return 2;
  }
  if (fseek(f, 0, SEEK_END) != 0 || (size = ftell(f)) < 0 || size > 4194304 ||
      fseek(f, 0, SEEK_SET) != 0) {
    fclose(f);
    fprintf(stderr, "invalid replay size\n");
    return 2;
  }
  data = (char *)malloc((size_t)size + 1);
  if (!data) {
    fclose(f);
    return 2;
  }
  if (fread(data, 1, (size_t)size, f) != (size_t)size) {
    free(data);
    fclose(f);
    return 2;
  }
  fclose(f);
  data[size] = 0;
  s = mb_replay_json(data, (size_t)size, &r);
  free(data);
  if (s != MB_OK) {
    fprintf(stderr, "replay rejected: %s\n", mb_status_name(s));
    return 1;
  }
  if (printf("replay valid: %llu pairs, %s\n",
             (unsigned long long)r.pair_count,
             mb_verdict_name(r.analysis.verdict)) < 0 ||
      fflush(stdout) != 0 || ferror(stdout)) {
    mb_report_free(&r);
    return 1;
  }
  mb_report_free(&r);
  return 0;
}
int main(int argc, char **argv) {
  MBConfig c = mb_default_config();
  MBReport report;
  Work baseline, candidate;
  int i;
  const char *revision = NULL, *replay = NULL;
  int64_t expected;
  MBStatus status;
  char *json;
  for (i = 1; i < argc; i++) {
    if (strcmp(argv[i], "--revision") == 0 && i + 1 < argc)
      revision = argv[++i];
    else if (strcmp(argv[i], "--pairs") == 0 && i + 1 < argc) {
      const char *text = argv[++i];
      const char *digit;
      char *end;
      unsigned long n;
      for (digit = text; *digit >= '0' && *digit <= '9'; digit++) {
      }
      if (digit == text || *digit != '\0') {
        fprintf(stderr, "invalid pair count\n");
        return 2;
      }
      errno = 0;
      n = strtoul(text, &end, 10);
      if (errno == ERANGE || end == text || *end || n < 1 || n > 10000) {
        fprintf(stderr, "invalid pair count\n");
        return 2;
      }
      c.pairs = (size_t)n;
    } else if (strcmp(argv[i], "--replay") == 0 && i + 1 < argc)
      replay = argv[++i];
    else {
      fprintf(stderr, "usage: array-microbenchmark --revision REV [--pairs N] "
                      "| --replay FILE\n");
      return 2;
    }
  }
  if (replay)
    return read_replay(replay);
  if (!revision) {
    fprintf(stderr, "--revision is required\n");
    return 2;
  }
  prepare(&baseline, 0);
  prepare(&candidate, 1);
  status = mb_indexed_array(0, 1, &expected);
  if (status != MB_OK)
    return 2;
  status = mb_collect(&c, expected, mb_native_clock, NULL, work_batch,
                      &baseline, work_batch, &candidate, &report);
  if (status != MB_OK) {
    fprintf(stderr, "benchmark failed: %s\n", mb_status_name(status));
    return 1;
  }
  json = report_json(&report, revision, argc, argv);
  mb_report_free(&report);
  if (!json) {
    fprintf(stderr, "report allocation failed\n");
    return 2;
  }
  {
    MBReport checked;
    status = mb_replay_json(json, strlen(json), &checked);
    if (status != MB_OK) {
      free(json);
      fprintf(stderr, "report JSON validation failed: %s\n",
              mb_status_name(status));
      return 2;
    }
    mb_report_free(&checked);
  }
  if (fputs(json, stdout) == EOF || fflush(stdout) != 0 || ferror(stdout)) {
    free(json);
    return 1;
  }
  free(json);
  return 0;
}
