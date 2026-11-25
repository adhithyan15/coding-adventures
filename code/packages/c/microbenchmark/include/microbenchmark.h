/* Paired microbenchmarks. Callbacks are borrowed and invoked synchronously. */
#ifndef CA_MICROBENCHMARK_H
#define CA_MICROBENCHMARK_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef enum {
  MB_OK = 0,
  MB_ARGUMENT,
  MB_LIMIT,
  MB_MEMORY,
  MB_CLOCK,
  MB_WORKLOAD,
  MB_CHECKSUM,
  MB_ANALYSIS,
  MB_JSON
} MBStatus;
typedef enum { MB_BASELINE_FIRST = 0, MB_CANDIDATE_FIRST = 1 } MBOrder;
typedef enum {
  MB_INCONCLUSIVE = 0,
  MB_FASTER,
  MB_SLOWER,
  MB_EQUIVALENT
} MBVerdict;
typedef struct {
  int64_t baseline_ns, candidate_ns;
  MBOrder order;
} MBPair;
typedef struct {
  size_t pairs;
  unsigned confidence_percent;
  double minimum_factor;
  int64_t seed, target_ns, max_batch;
} MBConfig;
typedef struct {
  double ratio, low, high;
  int interval_available;
  MBVerdict verdict;
} MBAnalysis;
typedef void *(*MBAllocate)(size_t bytes, void *context);
typedef void (*MBRelease)(void *pointer, void *context);
/* Supplied callbacks must return fresh, suitably aligned memory that release
 * can reclaim. Keep context alive through mb_report_free on returned reports. */
typedef struct {
  MBAllocate allocate;
  MBRelease release;
  void *context;
} MBAllocator;
typedef struct {
  MBConfig config;
  size_t batch_size, pair_count;
  MBPair *pairs;
  MBAnalysis analysis;
  MBRelease release_pairs;
  void *allocator_context;
} MBReport;
typedef int (*MBClock)(void *context, int64_t *nanoseconds);
typedef int (*MBBatch)(void *context, size_t operations, int64_t *checksum);
const char *mb_status_name(MBStatus status);
const char *mb_verdict_name(MBVerdict verdict);
MBConfig mb_default_config(void);
MBStatus mb_analyze(const MBPair *pairs, size_t count,
                    unsigned confidence_percent, double minimum_factor,
                    MBAnalysis *out);
MBStatus mb_analyze_with_allocator(const MBPair *pairs, size_t count,
                                   unsigned confidence_percent,
                                   double minimum_factor, MBAnalysis *out,
                                   const MBAllocator *allocator);
/* On success out owns pairs and must be released with mb_report_free; that
 * function tolerates repeated release. On failure out is zeroed. */
MBStatus mb_collect(const MBConfig *config,
                    int64_t expected_checksum_per_operation, MBClock clock,
                    void *clock_context, MBBatch baseline,
                    void *baseline_context, MBBatch candidate,
                    void *candidate_context, MBReport *out);
MBStatus mb_collect_with_allocator(
    const MBConfig *config, int64_t expected_checksum_per_operation,
    MBClock clock, void *clock_context, MBBatch baseline,
    void *baseline_context, MBBatch candidate, void *candidate_context,
    const MBAllocator *allocator, MBReport *out);
void mb_report_free(MBReport *report);
MBStatus mb_indexed_array(int candidate_iteration, size_t operations,
                          int64_t *checksum);
/* Replay accepts exactly the report schema; out owns no data after failure. */
MBStatus mb_replay_json(const char *json, size_t length, MBReport *out);
MBStatus mb_replay_json_with_allocator(const char *json, size_t length,
                                       const MBAllocator *allocator,
                                       MBReport *out);
/* Native clock adapter. Returns 0 on any OS clock/query/range error. */
int mb_native_clock(void *unused, int64_t *nanoseconds);
#ifdef __cplusplus
}
#endif
#endif
