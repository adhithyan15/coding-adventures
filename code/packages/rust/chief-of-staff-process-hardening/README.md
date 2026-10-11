# chief-of-staff-process-hardening

Process hardening for the D18 Chief daemon (D18S S-I5, step 6 P2.6a).

The daemon is the supervisor, and today also the broker. It holds every
agent's channel keys, unsealed vault material while a lease is being
served, and the audit log. A core dump would write all of that to disk. A
debugger running as the same user could read it live. An agent runs as the
same user unless principal separation is configured.

```rust,no_run
fn main() {
    // First, before any key exists. Refuse to start rather than run exposed.
    match chief_of_staff_process_hardening::suppress_core_dumps() {
        Ok(protection) => {
            for missing in protection.missing {
                eprintln!("not yet hardened on this platform: {missing}");
            }
        }
        Err(error) => {
            eprintln!("refusing to start: {error}");
            std::process::exit(1);
        }
    }
    // ... the daemon proper
}
```

## What it sets

| Platform | Measure | What it stops |
|---|---|---|
| every Unix | `RLIMIT_CORE` = 0, soft **and** hard | a core file on crash; with the hard limit at zero, it cannot be raised again |
| Linux | `prctl(PR_SET_DUMPABLE, 0)` | core dumps regardless of limit; ptrace and `/proc/<pid>/mem` from any process of the same user (the kernel makes `/proc/<pid>` root-owned) |
| macOS | `ptrace(PT_DENY_ATTACH)` | a debugger attaching later |
| Windows | nothing yet | reported in `missing`: the process DACL is D18S step 8 |

Every measure that can be read back is, and a mismatch is an error (S-P3:
fail loudly). `PT_DENY_ATTACH` has no getter. The `CoreDumpProtection` it
returns is the record of what is actually in force, and of what is still
missing. On macOS, that includes the Hardened Runtime without
`get-task-allow`: that signing setting, not `PT_DENY_ATTACH`, is what stops
`task_for_pid` memory reads.

## What it does not change

- **Agents' dumpability.** `exec` resets it for the new image, so an agent
  spawned by the daemon is dumpable again. Every child does inherit the zero
  core limit, with a hard limit it cannot raise.
- **The daemon's own `/proc/self`.** A non-dumpable process can still list
  its own descriptors. The spawn paths depend on this: spawn-isolation and
  the Linux sandbox's shim read `/proc/self/fd` in the forked children. A
  test checks it.

## Side effects to know about

- **The daemon cannot write its own `/proc/self` files** any more, and
  neither can its forked children before they exec. A non-dumpable process's
  `/proc/<pid>` belongs to root. Reading its own fd, task and exe entries
  still works, but writing `comm`, `oom_score_adj`, `uid_map` or `setgroups`
  fails with `EACCES`. Nothing in the daemon writes them today. Lowering the
  daemon's OOM score, or user-namespace setup, would have to happen before
  `suppress_core_dumps`, or some other way.
- **Under a debugger on macOS** (`lldb`), the daemon exits inside
  `PT_DENY_ATTACH` with status 45, without printing anything. That is the
  call doing its job.

## Tests

`process-hardening-probe` hardens itself and prints what the kernel reports.
The tests read that report from outside the probe, and check each value
against an unhardened control. The daemon's smoke test checks the running
daemon the same way.
