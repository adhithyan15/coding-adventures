---
category: Testing & coverage
---

# A worker failure test must observe the callback before requesting stop

Under parallel native-suite load, sleeping20ms did not guarantee that a worker
had run its first clock-failure tick. Cooperative stop correctly returned Ok,
so the expected error assertion failed. Wait for the callback's Release/Acquire
signal under the existing five-second deadline before stopping and joining.
Keep the precise error and callback assertions; a missing callback still fails.
Preserve the original failed receipt instead of treating a later run as its
acceptance. This changes fixture synchronisation, not production lifecycle.
