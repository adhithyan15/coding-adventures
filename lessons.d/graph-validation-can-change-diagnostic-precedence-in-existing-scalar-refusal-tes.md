---
category: Compiler / VM / language pipeline
---

# Graph validation can change diagnostic precedence in existing scalar refusal tests

Adding a pre-emission definite-assignment pass made malformed missing destinations
fail at a later read before scalar shape validation ran. Existing refusal tests
must still prove rejection, while expecting the actual diagnostic at the new
boundary. Likewise a formerly unsupported label now needs control-shape tests,
and an extra return is unreachable under the new CFG contract. Do not weaken
acceptance or remove negative cases just to preserve obsolete diagnostic order.
