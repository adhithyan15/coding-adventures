# hdd-simulator (C17)

A fixed, in-memory disk model implementing DB02's deterministic HDD profile.
It has 32 eight-byte sectors, a four-track head, and an integer simulated
clock. Reads and writes complete synchronously and return exact seek, rotation,
transfer, and total timing.

```c
#include "hdd_simulator.h"

HddSimulator *disk = NULL;
uint8_t sector[HDD_SECTOR_BYTES];
HddTiming timing;
if (hdd_create(&disk) == HDD_OK) {
    HddStatus status = hdd_read_sector(disk, 0, sector, sizeof sector, &timing);
    hdd_destroy(disk);
}
```

The opaque handle owns all medium bytes. Read and write use caller buffers with
explicit lengths, retain no pointers, and leave device state unchanged on any
refusal. `hdd_get_snapshot` copies the full state into detached caller storage.
The portable clock ceiling is `2^53-1`. This is an idealized teaching model,
not a filesystem, drive controller, cache, scheduler, real-time model, or
persistent medium.

See [DB02](../../../specs/DB02-hdd-simulator.md) and the [C profile](../../../specs/DB02a-hdd-simulator-c.md)
for exact mapping, status ordering, timing, ownership, and verification details.
The library has no runtime dependencies. Its native test adapter uses the
bounded JSON parser only to consume the canonical shared trace.

## Build

Run `sh BUILD` on POSIX or `BUILD_windows` from PowerShell. The strict ISO C17
runner tests the shared trace, boundary behavior, all 256 track/sector/phase
timing combinations, and deterministic independent-model histories with every
available GCC and Clang compiler.

`sh tools/fuzz.sh` optionally runs separate 100,000-input fixture and binary
history campaigns under Clang libFuzzer with ASan/UBSan. The strict bounded
adapter validates trace expectations against an independent tick-stepping
full-media model before runtime replay; see `tests/fuzz-report.md` for fixed
seeds, sanitizer results, and measured coverage.
