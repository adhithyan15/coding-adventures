---
category: CI & GitHub Actions
---

# A Windows BUILD script must not hard-code /c/ drive paths: after setup-ocaml, bash is Cygwin's

PR #18781 touched an OCaml package and an unrelated Rust crate
(`rust/document-ast`) landed in the same Windows build. The OCaml change made
the job run `ocaml/setup-ocaml`, which installs Cygwin and puts its `bin` on
PATH. From then on `bash ../_windows_cargo.sh` ran under Cygwin's bash, where
drive C is `/cygdrive/c`, not Git Bash's `/c`. The wrapper's hard-coded
`/c/Program Files (x86)/.../vswhere.exe` therefore "did not exist" and the
crate failed in 0.0s with `vswhere.exe not found`. The bug had been latent for
a long time: it only fires when a Windows Rust crate using the wrapper and an
OCaml package are affected by the same PR.

Fix: build every POSIX path from its Windows form with `cygpath -u`, which
both Git Bash and Cygwin ship and which answers in the running shell's own
spelling (`ProgramFiles(x86)` read via `printenv`, since bash cannot name a
variable with parentheses).

Do differently: in any shell script that runs on Windows CI, never write a
literal `/c/...` path. Convert from the Windows path with `cygpath`.
