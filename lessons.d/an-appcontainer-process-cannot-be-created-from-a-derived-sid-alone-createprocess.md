---
category: CI & GitHub Actions
---

# An AppContainer process cannot be created from a derived SID alone; CreateProcessW needs a registered profile

**What went wrong.** The D18S Windows applier (P2.8a) derived its
AppContainer SID with `DeriveAppContainerSidFromAppContainerName` and
registered no profile, so the agent would have no persistent storage
folder. The spec said "the kernel needs only the SID". It was never run on
Windows before review. On its first windows-2025 run, every spawn failed
with `CreateProcessW: The system cannot find the file specified (os error
2)`: `CreateProcessW` with `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`
looks the SID up among the registered profiles. The same run also found
two Unix-only test assumptions in crates that had never run there:

- `getaddrinfo("")` succeeds on Windows;
- a killed `std::process::Child` exits 1 on Windows, not with no code.

**Fix.** Register the profile just before `CreateProcessW`. Remove it
(folder and registry key) while the process is still suspended, and fail
closed if removal fails. Serialize the span. Refuse the empty host name in
the resolver itself, and make the kill exit code platform-aware in the
test.

**Then the environment.** With the profile registered, the next run
failed with `ERROR_ENVVAR_NOT_FOUND` (203). For an AppContainer,
`CreateProcessW` requires `LOCALAPPDATA` in an explicit environment block
and reroutes it (and `TEMP`/`TMP`) into the container's folder. A closed
environment must add it. Microsoft's own mxc project requires `SystemRoot`
and `LOCALAPPDATA` for the same reason. Each Win32 prerequisite surfaced
only after the previous one was fixed, one ~70-minute CI round each.
Search for prior art on the exact error before the first run, not after.

**Do differently.** A Win32 design claim ("X is enough") is a hypothesis
until a native run confirms it. Put a new Windows crate in the
`build-windows-os-suites` gate in its first push, so the first review
round already has native evidence. Expect the run to turn up Unix-only
assumptions in every other affected crate too.
