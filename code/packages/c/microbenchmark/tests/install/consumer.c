#include <microbenchmark.h>
#include <rank_statistics.h>
#include <string.h>
int main(void) {
  const char *json = "{\"workload\":\"indexed-array-sum-4096\",\"invocation\":[\"bench\"],\"revision\":\"installed\",\"runtime\":\"C17\",\"os\":\"test\",\"arch\":\"test\",\"config\":{\"pairs\":1,\"confidence_percent\":95,\"minimum_factor\":1.05,\"seed\":0,\"target_ns\":1,\"max_batch\":1},\"batch_size\":1,\"pairs\":[{\"baseline_ns\":100,\"candidate_ns\":100,\"order\":\"baseline-first\"}],\"analysis\":{\"ratio\":1,\"interval\":null,\"verdict\":\"inconclusive\"}}";
  double values[] = {1.0, 3.0, 2.0};
  double median = 0.0;
  MBReport report;
  if (rs_median(values, 3, &median) != RS_OK || median != 2.0)
    return 1;
  if (mb_replay_json(json, strlen(json), &report) != MB_OK)
    return 2;
  if (report.pair_count != 1 || report.analysis.ratio != 1.0) {
    mb_report_free(&report);
    return 3;
  }
  mb_report_free(&report);
  return 0;
}
