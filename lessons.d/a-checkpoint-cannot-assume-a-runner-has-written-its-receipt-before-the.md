---
category: Testing & coverage
---

# A checkpoint cannot assume a runner has written its receipt before the source freeze finishes

The pipeline red runner was still collecting its source freeze when a separate
checkpoint reader opened the not-yet-written JSON and failed with FileNotFoundError.
No issue or checkpoint mutation occurred. The running producer subsequently wrote
its receipt and all four genuine semantic reds were consumed; checkpointing then
succeeded. A future runner should write an explicit initial state before collecting
the freeze, and readers must distinguish receipt initialization from Cargo execution.
Do not describe a missing receipt as test success or silently restart a live producer.
