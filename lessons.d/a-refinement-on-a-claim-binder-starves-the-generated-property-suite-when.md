---
category: Rust
---

# A refinement on a claim binder starves the generated property suite when few values pass it, and a prediction read from generator source is a guess until you run it

VOS-012a, the first `-any` run of the evidence generator. Two things the contract text assumed were wrong.

1. `dispatch_covers_run` was written `forall mask: U8, t: U8 | t < 8u8. ...`. The exhaustive suite handles that (2,048 of 65,536 cases admitted), but the property suite draws `t` uniformly from 0..255 and retries a rejected draw at most 100 times (`max_retries` in `test-bootstrap`'s `pbt.rs`). At 3 percent acceptance, 100 consecutive rejections happen within a few iterations, so the suite reports `property-exhausted` and `cargo test` fails. The guard was also redundant (`mem` is false outside 0..7). Fix: drop a refinement the body already implies; if it is not implied, make the binder's own range the admissible set. VOS-D079 tracks the sampler.
2. VOS-D054 predicted, from reading `find_operation_in_expr`, that the `any` claims' negative controls would attach to `alpha_w` because the claims lead with `safe_dispatch(...)`. Running the generator showed they attach to `dispatch_img` and `dispatch_out`. A prediction about a tool's behaviour that was never run recorded a wrong "fix this" item in the backlog for a whole PR cycle.

Do instead: when a contract claim has a refinement, generate and run its property suite before the contract is final; when a backlog row says "predicted, not run", run it at the first opportunity and overwrite the row with the observed result.
