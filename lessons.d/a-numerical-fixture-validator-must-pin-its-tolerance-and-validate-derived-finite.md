---
category: TypeScript / JavaScript
---

# A numerical fixture validator must pin its tolerance and validate derived finiteness

A caller-controlled `absolute_tolerance` with only a `> 0` check can be raised until a dishonest oracle passes. Require the corpus's canonical tolerance in both schema and executable validator. Likewise, checking that inputs are finite is not enough: addition, multiplication, reductions, and finite differences can overflow into `Infinity` or `NaN`. Bound teaching inputs at the runtime boundary, verify arrays before invoking array methods, catch host-language numeric conversion overflow, and assert every derived output, gradient, score, and audit error remains finite. This was caught in the NN26 tensor-broadcasting pre-push review.
