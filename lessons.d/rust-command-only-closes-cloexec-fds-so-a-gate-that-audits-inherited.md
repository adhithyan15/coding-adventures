---
category: CI & GitHub Actions
---

# Rust Command only closes CLOEXEC fds, so a gate that audits inherited fds must close them in the launcher

The runtime capability gate (VER08b) passed locally but failed on CI with "no checks ran" and RCG009 for all 18 negative controls. The child's RCG011 audit found two plain pipes (fds 142, 145) leaked by the GitHub Actions runner; `std::process::Command` only closes O_CLOEXEC descriptors, so they reached the child. The audit was right to refuse; the launcher was wrong to pass them. Fix: close everything above fd 2 in `pre_exec` (`close_range`). Do: when a check asserts "only 0/1/2 are open", make the launcher enforce it, and reproduce CI by leaking fds locally (`exec 142< <(sleep 60)`) before running.
