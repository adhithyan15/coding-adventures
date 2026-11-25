#include "iso_test.h"
#include "rank_statistics.h"
#include <float.h>
#include <math.h>
#include <stddef.h>
#include <stdint.h>
static int coefficient_oracle(unsigned n, unsigned confidence, unsigned k) {
  uint64_t c = 1, sum = 0;
  unsigned j;
  for (j = 0; j < k; j++) {
    sum += c;
    c = c * (n - j) / (j + 1);
  }
  return 200u * sum <= (uint64_t)(100u - confidence) * (UINT64_C(1) << n);
}
static int sign_oracle(unsigned n, unsigned confidence, unsigned k) {
  uint64_t mask, assignments = UINT64_C(1) << n, tail = 0;
  for (mask = 0; mask < assignments; mask++) {
    unsigned heads = 0, j;
    for (j = 0; j < n; j++)
      heads += (unsigned)((mask >> j) & 1u);
    if (heads < k)
      tail++;
  }
  return 200u * tail <= (uint64_t)(100u - confidence) * assignments;
}
static void *never_allocate(size_t n, void *context) {
  (void)n;
  (void)context;
  return NULL;
}
static void ignore_release(void *p, void *context) {
  (void)p;
  (void)context;
}
int main(void) {
  static double oversized[RS_MAX_SAMPLES + 1];
  double v[] = {1.0, 2.0, 3.0, 4.0}, m = 0.0;
  RSInterval ci;
  size_t n;
  unsigned conf, k;
  ISO_CHECK(rs_median(v, 4, &m) == RS_OK);
  ISO_CHECK_EQ_DBL(m, 2.5, 0.0);
  ISO_CHECK(rs_median(v, 0, &m) == RS_EMPTY);
  ISO_CHECK(rs_median(NULL, 1, &m) == RS_ARGUMENT);
  ISO_CHECK(rs_median(oversized, RS_MAX_SAMPLES + 1, &m) == RS_LIMIT);
  {
    RSAllocator fail = {never_allocate, ignore_release, NULL};
    ISO_CHECK(rs_median_with_allocator(v, 4, &m, &fail) == RS_MEMORY);
    ISO_CHECK(rs_median_interval_with_allocator(v, 4, 95, &ci, &fail) ==
              RS_MEMORY);
    fail.release = NULL;
    ISO_CHECK(rs_median_with_allocator(v, 4, &m, &fail) == RS_ARGUMENT);
    fail.allocate = NULL;
    fail.release = ignore_release;
    ISO_CHECK(rs_median_interval_with_allocator(v, 4, 95, &ci, &fail) ==
              RS_ARGUMENT);
  }
  {
    double x[] = {-DBL_MAX, DBL_MAX};
    ISO_CHECK(rs_median(x, 2, &m) == RS_OK);
    ISO_CHECK(m == 0.0);
  }
  {
    double unit = nextafter(0.0, 1.0);
    double x1[] = {unit, 2.0 * unit};
    double x2[] = {-2.0 * unit, -unit};
    double x3[] = {nextafter(DBL_MIN, 0.0), DBL_MIN};
    double x4[] = {-DBL_MIN, -nextafter(DBL_MIN, 0.0)};
    double x5[] = {DBL_MAX, DBL_MAX};
    double x6[] = {-DBL_MAX, -DBL_MAX};
    ISO_CHECK(rs_median(x1, 2, &m) == RS_OK && m == 2.0 * unit);
    ISO_CHECK(rs_median(x2, 2, &m) == RS_OK && m == -2.0 * unit);
    ISO_CHECK(rs_median(x3, 2, &m) == RS_OK && m == DBL_MIN);
    ISO_CHECK(rs_median(x4, 2, &m) == RS_OK && m == -DBL_MIN);
    ISO_CHECK(rs_median(x5, 2, &m) == RS_OK && m == DBL_MAX);
    ISO_CHECK(rs_median(x6, 2, &m) == RS_OK && m == -DBL_MAX);
  }
  {
    double x[] = {1.0, NAN};
    ISO_CHECK(rs_median(x, 2, &m) == RS_NONFINITE);
  }
  for (n = 1; n <= 20; n++)
    for (conf = 1; conf <= 99; conf++) {
      double sample[20];
      for (k = 0; k < n; k++)
        sample[k] = (double)k;
      ISO_CHECK(rs_median_interval(sample, n, conf, &ci) == RS_OK);
      {
        size_t expected = 0;
        for (k = 1; k <= n / 2; k++) {
          int qualifies = n <= 10 ? sign_oracle((unsigned)n, conf, k)
                                  : coefficient_oracle((unsigned)n, conf, k);
          if (n <= 10)
            ISO_CHECK(qualifies == coefficient_oracle((unsigned)n, conf, k));
          if (qualifies)
            expected = k;
          else
            break;
        }
        ISO_CHECK(ci.available == (expected != 0));
        if (expected) {
          ISO_CHECK_EQ_DBL(ci.low, (double)(expected - 1), 0.0);
          ISO_CHECK_EQ_DBL(ci.high, (double)(n - expected), 0.0);
        }
      }
    }
  ISO_CHECK(rs_median_interval(v, 4, 0, &ci) == RS_ARGUMENT);
  {
    double duplicate[] = {7.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0, 7.0};
    ISO_CHECK(rs_median_interval(duplicate, 8, 95, &ci) == RS_OK);
    ISO_CHECK(ci.available);
    ISO_CHECK(ci.low == 7.0 && ci.high == 7.0);
  }
  {
    static double max_sample[10000];
    for (n = 0; n < 10000; n++)
      max_sample[n] = (double)n;
    ISO_CHECK(rs_median_interval(max_sample, 10000, 95, &ci) == RS_OK);
    ISO_CHECK(ci.available);
    ISO_CHECK(ci.low >= 0.0 && ci.high < 10000.0);
  }
  return ISO_TEST_RESULT();
}
