## [0.1.0] - UNRELEASED

### Added (KILN04 D48 step 6 prerequisite: biased exec-head differential; test-only)

- **Biased generation mode** (`internal/traced/exechead_biased_test.go`,
  `TestDifferentialExecHeadBiased`). Actions whose only would-be refusal is an exec
  under the fresh tmpfs, with an honest `.exechead` sidecar, so the classifier believes
  the head and the read-set oracle compares what it built. Covers tmpfs ELFs (static,
  loader in the checkout, host loader, rewritten after the exec), several execs per
  pid with failed ones, up to four pids, the init anchor and host/tmpfs execs mixed in
  one pid. 800 of 800 default seeds reach the read-set oracle honest and reason-free
  (the random generator: 1 of 4000); the floor is 700.
- Every sidecar perturbation runs against every biased case, with floors on the
  applied-and-refused counts, and three new ones (wrong syscall, unknown init pid, a
  confirmed `#!` head) plus a result-less (`= ?`) exec. The model records the look at
  an in-checkout PT_INTERP loader. No production change; the default package test
  grows by about 1.3 s of 7.9 s.
