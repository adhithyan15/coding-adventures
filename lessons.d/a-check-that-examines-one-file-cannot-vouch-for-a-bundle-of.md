# A check that examines one file cannot vouch for a bundle of 159

Security review of the Qt payload found that the relocatability and signature
gates ran on `Contents/MacOS/<executable>` and nothing else. The failure they
were written for is that `macdeployqt` rewrites **the dylibs'** install names
and invalidates **their** signatures — so the check examined the one file least
likely to be wrong. The bundle holds 159 Mach-O files; one of them still
pointing at `/opt/homebrew/opt/qtbase/...` passes every gate and fails to launch
for every downloader. The commit message said "the archiver refuses any payload
that still carries an absolute dependency"; what it refused was a payload whose
*top-level executable* did. Prose outrunning code, again.

Three more from the same review, all verified before fixing:

- **`is_code_signed` returned `True` for any universal binary without looking
  inside** — a fat header followed by zeros passed. A Qt build for
  `arm64;x86_64` *is* universal, which is the normal shape for a public macOS
  release, so the gate would have silently stopped asserting anything the
  moment the build went universal. The comment justified it by citing a "bundle
  check" that did not exist.
- **Prefix matching without normalisation is a spelling check, not a resolution
  check.** `@executable_path/../../../../opt/homebrew/...`,
  `/usr/lib/../../opt/homebrew/...`, and even `@executable_pathological/` all
  passed a plain `startswith`.
- **Unvalidated Mach-O headers are a resource attack.** A chain of fat headers
  turned a 2 MB file into 2 GB of RSS; a `cmdsize` of zero never advances the
  cursor, so a 64-byte file stalled the parser for minutes.

But the fix for the second one **rejected every valid Qt bundle**, and only
running it against a real deployed app caught that: the standard macdeployqt
install name is `@executable_path/../Frameworks/QtCore.framework/QtCore`, which
climbs one level from `Contents/MacOS` and lands *inside* the bundle. Treating
any `..` as an escape is wrong; the bound is how deep the binary sits in the
bundle. **A hardening change needs its false-positive direction tested against
real data as carefully as its true-positive direction against fixtures** — a
release lane that refuses correct payloads fails just as completely as one that
accepts broken ones.

Same lesson again on `LC_RPATH`: refusing any run path pointing outside the
bundle failed the real bundle over
`/opt/homebrew/Cellar/dbus/1.16.2_1/lib` in `libdbus-1.3.dylib`. Measuring
settled it — 4 binaries carry an outside run path and **none** has an `@rpath`
dependency, so dyld never consults it. The rule now fires only when something
actually resolves through it.

And one on method: a mutation test that reports OK proves nothing until you
confirm the mutation applied. Mine used the wrong indentation, silently matched
nothing, and "passed". When redone properly the guard turned out to be
redundant in `dylib_dependencies` (a second check caught the same input) and
load-bearing in `is_code_signed`, where its removal hung on a 64-byte file. Two
different answers that the vacuous run had collapsed into one false one.
