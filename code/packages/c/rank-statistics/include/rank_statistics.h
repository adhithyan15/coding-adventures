/* Rank statistics for finite binary64 samples. All inputs are borrowed. */
#ifndef CA_RANK_STATISTICS_H
#define CA_RANK_STATISTICS_H
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
#define RS_MAX_SAMPLES ((size_t)10000)
typedef enum {
  RS_OK = 0,
  RS_ARGUMENT,
  RS_EMPTY,
  RS_NONFINITE,
  RS_LIMIT,
  RS_MEMORY
} RSStatus;
typedef struct {
  double low, high;
  int available;
} RSInterval;
typedef void *(*RSAllocate)(size_t bytes, void *context);
typedef void (*RSRelease)(void *pointer, void *context);
typedef struct {
  RSAllocate allocate;
  RSRelease release;
  void *context;
} RSAllocator;
const char *rs_status_name(RSStatus status);
/* values[0..count) must be readable and unchanged during the call. */
RSStatus rs_median(const double *values, size_t count, double *out);
RSStatus rs_median_interval(const double *values, size_t count,
                            unsigned confidence_percent, RSInterval *out);
/* Testable allocation boundary; both callbacks must be supplied together.
 * allocate returns fresh, suitably aligned storage which release can reclaim.
 */
RSStatus rs_median_with_allocator(const double *values, size_t count,
                                  double *out, const RSAllocator *allocator);
RSStatus rs_median_interval_with_allocator(const double *values, size_t count,
                                           unsigned confidence_percent,
                                           RSInterval *out,
                                           const RSAllocator *allocator);
#ifdef __cplusplus
}
#endif
#endif
