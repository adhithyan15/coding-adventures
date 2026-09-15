# `chief-of-staff-cli`: fifth recurrence of native `Path::join` used to build a path for a *different target platform* than the host

Once `chief-of-staff-daemon-credential` stopped failing, `chief-of-staff-cli`
and `chief-of-staff-daemon` came off `DEP-SKIP` and ran for the first time
this rescue — immediately surfacing `derives_reviewable_requests_for_every_
native_supervisor`, failing on Windows CI with
`left: "/opt/chief\\chief-of-staff-daemon"` vs.
`right: "/opt/chief/chief-of-staff-daemon"`.

`sibling_executable()`'s non-Windows branch built the launchd/systemd sibling
path with `Path::new(current).parent().join("chief-of-staff-daemon")`. The
test deliberately feeds a POSIX `current_executable` (`"/opt/chief/chief-of-
staff"`) for the `Launchd` platform variant *unconditionally* — this
function must produce a POSIX path regardless of which OS built and ran the
test binary, because the target is always a macOS/Linux daemon. `Path::join`
uses the **host** platform's preferred separator, so on a Windows-built test
binary it silently produced a mixed `/opt/chief\chief-of-staff-daemon`.

This is the same family as the four earlier `is_absolute()` bugs in this
rescue (`ir_to_jvm_class_file`, `chief-of-staff-daemon-service-files`,
storage paths in general): **`std::path::Path`/`PathBuf` encode the host's
path conventions, not the path string's own.** Whenever code manipulates a
path that is contractually for a specific *other* platform — a POSIX daemon
supervisor path, a Windows registry path, anything not "this process's own
filesystem" — reach for explicit string splitting/joining (as the sibling
`windows: bool` branch in this very function already did with
`rsplit_once('\\')`), never `Path`. The fix (`unix_sibling_path()`) hand-rolls
`Path`'s own POSIX parent semantics (root normalizes to `/`, a trailing slash
is insignificant, a bare relative name has no usable parent) with plain
string operations, verified against all three cases already covered by
`invalid_sibling_paths_fail_stably` plus the new-to-Windows-CI
`derives_reviewable_requests_for_every_native_supervisor`.

Tell: whenever a test constructs an `InstallEnvironment`/similar struct with
a path literal that does **not** match `cfg!(windows)` (a `/`-style path
fed into a test that runs unconditionally, not gated by `#[cfg(unix)]`), any
`Path`-based manipulation of that literal downstream is host-dependent and
therefore Windows-CI-only-broken — grep for `Path::new(` / `.parent()` /
`.join(` near a hardcoded POSIX or Windows path literal before trusting it
is portable.
