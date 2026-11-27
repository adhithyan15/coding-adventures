# nand-flash (C17)

An in-memory logical SLC NAND model implementing DB01. It models full-page
program-once behavior and block erase counts; it is not a device controller,
FTL, wear-leveler, ECC implementation, or persistent store.

```c
#include "nand_flash.h"

NandFlash *flash = NULL;
uint8_t page[512] = {0};
NandFlashStatus status = nand_flash_create(4, 32, sizeof page, false, 0, &flash);
if (status == NAND_FLASH_OK) {
    status = nand_flash_program(flash, 0, 0, page, sizeof page);
    nand_flash_destroy(flash);
}
```

The handle is opaque and owns its page bytes, markers, and counters. Create
writes the output handle only on success (and clears the valid output slot
first). `destroy(NULL)` is safe. Geometry getters return zero for a null handle;
valid geometry is always positive. Read and program copy caller bytes, so the
caller retains ownership and may reuse buffers after return. Read destinations
are untouched on errors. Caller pointers must designate valid buffers of the
declared size; arbitrary pointer validity cannot be checked in C.

The accepted ranges and error messages match [DB01](../../../specs/DB01-nand-flash.md).
See [the C profile](../../../specs/DB01a-nand-flash-c.md) for argument
precedence, ownership details, and the test evidence boundary. The strict
test-only fixture adapter uses audited `bounded-json` to reject duplicate and
unknown fields, then checks fixture outcomes against an independent byte-array
model before it replays through the C API. The NAND runtime has no JSON
dependency.

## Build

Run `sh BUILD` on POSIX or `BUILD_windows` from PowerShell. The strict ISO C17
runner uses all available GCC and Clang compilers and consumes the shared trace,
native boundary tests, and deterministic independent-model history.

The optional Clang/libFuzzer lane is independent of both normal build fronts:

```sh
CLANG=/path/to/clang LLVM_PROFDATA=/path/to/llvm-profdata \
  LLVM_COV=/path/to/llvm-cov sh tools/fuzz.sh
```

It runs 100,000 fixture-vector and 100,000 binary-history mutations in separate
scratch corpora under ASan/UBSan, then writes LLVM line/branch coverage receipts.
Malformed and out-of-domain vectors are expected inputs. A crash records a
sanitizer finding or a disagreement with the model. The history oracle checks
status messages, all pages, markers, erase counts, and caller output sentinels
after each command. See the [campaign report](tests/fuzz-report.md).
