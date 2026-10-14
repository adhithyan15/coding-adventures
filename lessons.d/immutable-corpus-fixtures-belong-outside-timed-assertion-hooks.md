---
category: Testing & coverage
---

# Immutable corpus fixtures belong outside timed assertion hooks

Run 38163286318 passed 207 human-language test files but failed the modality
manifest's 30-second setup hook. The hook only parsed immutable real lessons and
built the shared manifest; no assertion was slow or incorrect. Initialize those
read-only fixtures once when the test module loads, as the other corpus tests do,
while keeping actual assertions and CLI checks at their configured deadlines.
Retain live filesystem reads and cleanup hooks for fixtures that tests mutate.
Verify with the normal configuration, without a command-line timeout override.

Once the hook was fixed, the local run exposed repeated committed-manifest reads
inside assertions: one test loaded the same owners twice and took 122.6 seconds;
the live CLI assertion plus another owner load took 97.6 seconds. Cache the
immutable committed owners alongside the lessons, preserve their equality checks,
and keep the live CLI check within an explicit integration budget.
Use Windows junctions for directory-link fixtures when validation lacks symlink
creation privileges; preserve both dangling-target and linked-parent rejection.

The remaining live CLI check still took 74.2 seconds. It built the identical
manifest twice, once for validation and again for emitted owner bytes. Reuse the
already built manifest for those bytes, preserving every byte comparison,
identity check, owner reconstruction and recovery boundary. Fix repeated product
work instead of moving the live CLI check out of its deadline.

After that fix it still measured 66.1 seconds reading and validating the entire
direct-owner corpus. Give this one live filesystem integration test a measured
120-second budget in the test source, retaining the ordinary 30-second defaults.
Avoid command-line timeout overrides, unbounded live CLI setup and weakened
owner/path checks as substitutes for an honest integration cost.
