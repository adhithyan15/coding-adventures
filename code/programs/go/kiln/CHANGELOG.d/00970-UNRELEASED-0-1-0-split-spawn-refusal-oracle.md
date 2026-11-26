# KILN: account for resumed spawn PID generations in the test oracle

The real-strace dying-call test oracle now pairs unfinished and resumed
`clone`, `clone3`, `fork`, and `vfork` records before accepting a later exit for
the same numeric PID. Split records are matched by parent PID and syscall, and
candidate results are validated with the production trace parser. Production
trace parsing and its refusal rules are unchanged.
