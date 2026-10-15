---
category: Testing & coverage
---

# Read the checked geometry contract before naming a numeric cap

The first Railroad arc reconciliation spec inferred a16384 derived-layout cap
from other graphics evidence. The checked Railroad API actually checks finite
coordinates and nonnegative rectangle dimensions; its retained valid cases
include font/spacing65536 and larger finite layouts. Read the exact admission
loop and existing controls before naming a cap. Correct the spec before test
or production changes, preserve the failed assumption, and distinguish layout
admission from downstream raster surface/resource limits.
