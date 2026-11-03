## 0.1.1

- Enforce TF01's six registered failure IDs at the TAP serialization boundary,
  preserving source compatibility of Verdict::fail and direct Fail construction.
  Unknown IDs produce a fixed bail-out before their result/YAML is written;
  their ID/message is hidden, remaining callbacks stop, prior counts remain
  accurate, and the process exits nonzero. Writer errors still propagate.
- Expose verdict::is_registered_failure_id and test all valid IDs, malformed
  constructor/direct values, hostile text, early stop and valid-prefix counts.
- Extend the independently checked self-corpus from 72 to 85 cases with failure
  vocabulary membership. Require all registered IDs and an unregistered rejection.
- Generate canonical LF transport bytes on Windows as well as Unix; test the
  actual bytes rather than a newline-normalizing text read.
