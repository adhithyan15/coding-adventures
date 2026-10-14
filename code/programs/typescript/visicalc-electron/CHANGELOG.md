# Changelog

## Unreleased

- Override optional global-agent with 4.1.3, preserving proxy bootstrap while removing the vulnerable roarr / sprintf-js chain (GHSA-hp3w-g68c-fv3c).
- Require Electron 43.7.9 or later in the 43.x line, matching the patched lockfile while reconciling concurrent dependency updates.
