/* Exact binomial rank arithmetic, bounded to VER07's 10,000 observations. */
#include "rank_statistics.h"
#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* 10,001 bits cover 2^n and 200*sum(C(n,j)) for n <= 10,000. */
#define LIMBS 314u
typedef struct {
  uint32_t d[LIMBS];
} Big;
static void zero(Big *a) { memset(a, 0, sizeof(*a)); }
static void set_u32(Big *a, uint32_t x) {
  zero(a);
  a->d[0] = x;
}
static int add(Big *a, const Big *b) {
  uint64_t carry = 0;
  size_t i;
  for (i = 0; i < LIMBS; i++) {
    uint64_t x = (uint64_t)a->d[i] + b->d[i] + carry;
    a->d[i] = (uint32_t)x;
    carry = x >> 32;
  }
  return carry == 0;
}
static int mul_small(Big *a, uint32_t m) {
  uint64_t carry = 0;
  size_t i;
  for (i = 0; i < LIMBS; i++) {
    uint64_t x = (uint64_t)a->d[i] * m + carry;
    a->d[i] = (uint32_t)x;
    carry = x >> 32;
  }
  return carry == 0;
}
static int div_small(Big *a, uint32_t divisor) {
  uint64_t rem = 0;
  size_t i = LIMBS;
  if (!divisor)
    return 0;
  while (i--) {
    uint64_t cur = (rem << 32) | a->d[i];
    a->d[i] = (uint32_t)(cur / divisor);
    rem = cur % divisor;
  }
  return rem == 0;
}
static int cmp(const Big *a, const Big *b) {
  size_t i = LIMBS;
  while (i--) {
    if (a->d[i] < b->d[i])
      return -1;
    if (a->d[i] > b->d[i])
      return 1;
  }
  return 0;
}
static int set_shifted_u32(Big *a, uint32_t value, size_t bits) {
  size_t word = bits / 32u;
  unsigned rem = (unsigned)(bits % 32u);
  uint64_t shifted = (uint64_t)value << rem;
  zero(a);
  if (word >= LIMBS)
    return 0;
  a->d[word] = (uint32_t)shifted;
  if ((shifted >> 32) != 0) {
    if (word + 1 >= LIMBS)
      return 0;
    a->d[word + 1] = (uint32_t)(shifted >> 32);
  }
  return 1;
}
static int finite_sample(const double *v, size_t n) {
  size_t i;
  for (i = 0; i < n; i++)
    if (!isfinite(v[i]))
      return 0;
  return 1;
}
static int cmp_double(const void *a, const void *b) {
  double x = *(const double *)a, y = *(const double *)b;
  return (x > y) - (x < y);
}
const char *rs_status_name(RSStatus s) {
  switch (s) {
  case RS_OK:
    return "ok";
  case RS_ARGUMENT:
    return "argument";
  case RS_EMPTY:
    return "empty";
  case RS_NONFINITE:
    return "nonfinite";
  case RS_LIMIT:
    return "limit";
  case RS_MEMORY:
    return "memory";
  default:
    return "unknown";
  }
}
static void *default_allocate(size_t n, void *context) {
  (void)context;
  return malloc(n);
}
static void default_release(void *p, void *context) {
  (void)context;
  free(p);
}
static int valid_allocator(const RSAllocator *a) {
  return !a || (a->allocate && a->release);
}
static void *alloc_with(const RSAllocator *a, size_t n) {
  return a ? a->allocate(n, a->context) : default_allocate(n, NULL);
}
static void release_with(const RSAllocator *a, void *p) {
  if (a)
    a->release(p, a->context);
  else
    default_release(p, NULL);
}
RSStatus rs_median_with_allocator(const double *values, size_t count,
                                  double *out, const RSAllocator *allocator) {
  double *copy;
  double a, b, sum;
  if (!out || (!values && count) || !valid_allocator(allocator))
    return RS_ARGUMENT;
  if (!count)
    return RS_EMPTY;
  if (count > RS_MAX_SAMPLES)
    return RS_LIMIT;
  if (!finite_sample(values, count))
    return RS_NONFINITE;
  if (count > SIZE_MAX / sizeof(double))
    return RS_LIMIT;
  copy = (double *)alloc_with(allocator, count * sizeof(double));
  if (!copy)
    return RS_MEMORY;
  memcpy(copy, values, count * sizeof(double));
  qsort(copy, count, sizeof(double), cmp_double);
  if (count & 1u)
    *out = copy[count / 2];
  else {
    a = copy[count / 2 - 1];
    b = copy[count / 2];
    /* Sum first when representable: it preserves correct tie-to-even
       rounding at the subnormal boundary. If the sum overflows, halving
       each finite operand keeps the mean finite. */
    sum = a + b;
    *out = sum / 2.0;
    if (!isfinite(sum))
      *out = a / 2.0 + b / 2.0;
  }
  release_with(allocator, copy);
  return isfinite(*out) ? RS_OK : RS_NONFINITE;
}
RSStatus rs_median(const double *values, size_t count, double *out) {
  return rs_median_with_allocator(values, count, out, NULL);
}
RSStatus rs_median_interval_with_allocator(const double *values, size_t count,
                                           unsigned confidence, RSInterval *out,
                                           const RSAllocator *allocator) {
  double *copy;
  Big coefficient, tail, left, right;
  size_t k, best = 0;
  if (!out || (!values && count) || !valid_allocator(allocator))
    return RS_ARGUMENT;
  out->available = 0;
  out->low = out->high = 0.0;
  if (!count)
    return RS_EMPTY;
  if (count > RS_MAX_SAMPLES)
    return RS_LIMIT;
  if (confidence < 1 || confidence > 99)
    return RS_ARGUMENT;
  if (!finite_sample(values, count))
    return RS_NONFINITE;
  if (count > SIZE_MAX / sizeof(double))
    return RS_LIMIT;
  copy = (double *)alloc_with(allocator, count * sizeof(double));
  if (!copy)
    return RS_MEMORY;
  memcpy(copy, values, count * sizeof(double));
  qsort(copy, count, sizeof(double), cmp_double);
  set_u32(&coefficient, 1);
  zero(&tail);
  if (!set_shifted_u32(&right, 100u - confidence, count)) {
    release_with(allocator, copy);
    return RS_LIMIT;
  }
  /* For k=1..floor(n/2), tail is sum(C(n,j), j=0..k-1). */
  for (k = 1; k <= count / 2; k++) {
    if (!add(&tail, &coefficient)) {
      release_with(allocator, copy);
      return RS_LIMIT;
    }
    left = tail;
    if (!mul_small(&left, 200u)) {
      release_with(allocator, copy);
      return RS_LIMIT;
    }
    if (cmp(&left, &right) <= 0)
      best = k;
    else
      break;
    if (k < count / 2) {
      /* C(n,k) = C(n,k-1)*(n-k+1)/k, with exact division. */
      if (!mul_small(&coefficient, (uint32_t)(count - k + 1)) ||
          !div_small(&coefficient, (uint32_t)k)) {
        release_with(allocator, copy);
        return RS_LIMIT;
      }
    }
  }
  if (best) {
    out->available = 1;
    out->low = copy[best - 1];
    out->high = copy[count - best];
  }
  release_with(allocator, copy);
  return RS_OK;
}
RSStatus rs_median_interval(const double *values, size_t count,
                            unsigned confidence, RSInterval *out) {
  return rs_median_interval_with_allocator(values, count, confidence, out,
                                           NULL);
}
