# storage-wal-checkpoint-slot

Stages a fixed DBRF checkpoint frame by setting its reserved publication guard while preserving the checksum for the eventual zero-guard frame. Inspection distinguishes vacant, staged, and published slots and decodes only the latter.

The pure slot API models a single fixed-width frame. Physical staging tests separately exercise the inactive HDD slot's guard-first, body-second, guard-clear-last sequence with byte-prefix cuts.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
