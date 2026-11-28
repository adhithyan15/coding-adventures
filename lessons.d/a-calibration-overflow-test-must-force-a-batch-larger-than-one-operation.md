---
category: Testing & coverage
---

# A calibration overflow test must force a batch larger than one operation

An initial overflow case used a target reached at batch size one, so calibration never multiplied the maximum checksum by a larger count and the intended overflow branch was not exercised. Choose a fake-clock duration and target that require calibration to double the batch, then assert collection fails before timing pairs begin.
The collector first checks a one-operation batch. An overflow test whose
checksum is `maxBound` therefore passes warmup and calibration if the target
duration is reached at batch size one. Set the fake clock/target so calibration
must double the batch; then expected-checksum multiplication reaches the
overflow branch under test.
