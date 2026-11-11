---
category: CI & GitHub Actions
---

# A KILL_PROCESS seccomp profile for docker run must allow runc's own post-filter calls (openat2), and the failure surfaces as a misleading netns bind-mount error

While building the VER08b runtime capability gate, a generated Docker seccomp
profile (`defaultAction: SCMP_ACT_KILL_PROCESS`, allowlist = loader start-up +
the test binary's calls) made `docker run` fail before the program ran:

    docker: Error response from daemon: bind-mount /proc/<pid>/ns/net ->
    /var/run/docker/netns/...: no such file or directory

That message says nothing about seccomp. It appears because `runc init` was
killed by the filter (it installs the profile and keeps making calls until
`execve`), so the pid whose netns Docker then tries to bind-mount no longer
exists. The same profile with `SCMP_ACT_ERRNO` ran fine, which is the tell: the
filter, not the network, is the problem.

Bisecting (drop one candidate syscall at a time from a generous list, rerun
`docker run ... /bin/echo hi`, keep the drop if it still works) showed the only
missing call on Docker 29 / runc was `openat2` (runc's safe `/proc` access).

Also learned in the same change:

- A binary that installs its own stricter filter from inside the container
  makes the *container* profile need `prctl`, `seccomp` and `landlock_*` too.
- Docker injects `PATH`, `HOSTNAME` and `HOME` into every container; an
  environment audit inside the container must scrub or allow exactly those.
- LLVM-instrumented binaries set `__LLVM_PROFILE_RT_INIT_ONCE` in their own
  environment after `exec`, which an in-process environment audit will flag.
- The capability cage rejects manifests that combine untrusted inputs with
  external actuations, so a test that wants "every capability" must use
  several valid manifests, not one.

**What to do differently:** to debug a profile that kills the container
runtime, switch the default action to `SCMP_ACT_ERRNO`, confirm it runs, then
bisect the allowlist; do not chase the bind-mount error. Keep the discovered
start-up set as one reviewed constant (`STARTUP_SYSCALLS` in
`capability-runtime-conformance`) and re-bisect when Docker or runc is
upgraded.
