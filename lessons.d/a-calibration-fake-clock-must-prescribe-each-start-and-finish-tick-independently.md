---
category: C#
---

# A calibration fake clock must prescribe each start and finish tick independently

A fake duration tied to the most recent batch size failed because the collector
reads the start timestamp before invoking the callback. Updating the batch size
inside the callback only changed the finish timestamp, so the intended duration
was never measured. Use an explicit queue of start and finish ticks for each
warmup, calibration, and measured batch. This makes timing tests deterministic
and documents precisely which duration each phase should observe.
---
category: C#
---

# A calibration fake clock must prescribe each start and finish tick independently

A test tried to make fake duration depend on the most recent batch size, but
the collector reads the start tick before invoking the batch callback. The
batch-size update therefore affected only the finish tick, creating a duration
that was neither the intended batch duration nor the same for both variants.
Use a scripted tick sequence with explicit start/end differences for each
calibration and measured batch; this keeps the fake clock deterministic and
lets the test target a precise batch boundary.
