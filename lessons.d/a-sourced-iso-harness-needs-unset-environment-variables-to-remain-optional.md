---
category: BUILD files & dependency management
---

# A sourced ISO harness needs unset environment variables to remain optional

The C++ package runner used `set -u` before sourcing `iso-harness`. The shared
library reads optional variables such as `ISO_REQUIRE` without first assigning
them, so nounset mode stopped the runner before compiler discovery. Match the
existing package runners' `set -e` behavior, and let the harness handle
optional environment variables itself.
