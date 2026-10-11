---
category: Rust
---

# std::process::Command has not installed the command's environment when pre_exec hooks run

`chief-of-staff-linux-sandbox` execs the agent from inside its own
`pre_exec` hook, with `execveat(fd, "", argv, environ, AT_EMPTY_PATH)`. The
assumption was that `std` had already set `environ` to the command's
environment by then. It had not. The agent received the parent's **entire**
environment, including `GH_TOKEN` and the AWS keys, even after `env_clear()`.
The one test that compared the probe's environment against the command's
caught it.

What to do instead:
- A hook that execs by itself must build `envp` in the parent, from
  `Command::get_envs()`, and pass that.
- Treat the agent environment as a closed set: only variables set with
  `env`, never inherited ones. `get_envs` cannot tell you whether
  `env_clear` was called, so don't depend on it.
- Test the child's actual environment and argv against what the command
  set, with and without `env_clear`. A test that only checks that the child
  runs will not catch a leak.
