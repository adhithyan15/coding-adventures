---
category: Testing & coverage
---

# A sandbox test's unconfined control must be harmless when its syscall succeeds

The Linux applier's tests (`chief-of-staff-linux-sandbox`) run every probe
twice: confined, where seccomp must kill it, and unconfined, as the control
that shows the probe works. The first `mount` probe called
`mount("none", "/tmp", "tmpfs")`. Confined, it died with SIGSYS as intended.
Unconfined, in a container running as root, it **succeeded**. Each test run
stacked a fresh, empty tmpfs over the host's `/tmp`, in the shared mount
namespace, so it outlived the probe.

The symptoms looked unrelated:
- a temp file the test had just written read back as ENOENT;
- the session scratchpad and background-task output files "vanished";
- the harness blamed "another process's startup cleanup".

Nine runs left nine stacked mounts. Unmounting them needs a privilege the
session does not have.

What to do instead:
- Write every negative probe so that its syscall is harmless if it succeeds.
  Use a mount point and filesystem type that cannot exist, signal 0, and a
  forked child that `_exit`s at once.
- Remember that CI containers and dev containers often run as root. "This
  would need privilege" is not a safety argument.
- When files disappear partway through a run, check `/proc/self/mounts`
  before suspecting cleanup jobs.
