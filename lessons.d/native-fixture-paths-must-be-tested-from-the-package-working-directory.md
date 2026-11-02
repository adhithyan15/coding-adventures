---
category: Testing & coverage
---

# Native fixture paths must be tested from the package working directory

NN51's first native test launch used four parent components to reach specs
from code/packages/cpp/neural-framework. The actual package-to-code distance
is three. Run the executable from the same package cwd as both BUILD fronts;
check fixture-open failures explicitly rather than silently testing no data.

The preceding direct GCC executable launch also exited before printing because
its MinGW runtime directory was not on PATH. Put C:/Strawberry/c/bin on the
process PATH when running that compiler's binaries. A successful compile alone
is not runtime proof, and an empty launch failure is not a numerical failure.
