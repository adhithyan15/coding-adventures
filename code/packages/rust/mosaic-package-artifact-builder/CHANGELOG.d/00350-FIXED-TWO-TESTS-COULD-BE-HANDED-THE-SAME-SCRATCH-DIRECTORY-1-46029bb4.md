### Fixed -- two tests could be handed the same scratch directory (#15278)

`host_effect_tests` and `host_effect_overwrite_tests` each built a temp
directory name from the process id and a nanosecond timestamp. Tests in
one binary share the pid, and `SystemTime::now()` is not guaranteed to
advance between two threads reading it, so two tests could compute the
SAME path. `create_dir_all` is idempotent, so neither noticed.

Every one of those tests ends with `remove_dir_all(&root)`. The first to
finish deleted the other's fixture mid-run, and the survivor failed
reading a file it had written itself:

    Io("read /var/folders/.../host/qt/effects.h: No such file or directory")

Measured: **11 failures in 25 local runs** of the module before the fix,
**0 in 25** after. It was not a rare flake -- it lost roughly two runs in
five, and it blocked an unrelated PR by failing `build (macos-latest)`.

Both helpers now take their suffix from a process-wide
`AtomicUsize`, so the name is unique by construction rather than by
hoping the clock ticks between two threads.


