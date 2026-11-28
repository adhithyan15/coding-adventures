# hdd-write-crash-cuts (Go)

This standard-library-only package applies one explicit byte-prefix HDD
sector-write cut to a detached durable image. It has no timed device handle or
DB02 runtime dependency. The immutable input is thirty-two eight-byte sectors;
the output owns a separate image and reports persisted-prefix evidence apart
from the optional caller acknowledgement.

```go
import hddwritecrashcuts "github.com/adhithyan15/coding-adventures/code/packages/go/hdd-write-crash-cuts"

before := make([][]byte, hddwritecrashcuts.SectorCount)
for index := range before {
    before[index] = make([]byte, hddwritecrashcuts.BytesPerSector)
}
prefix := 3
result, err := hddwritecrashcuts.Apply(
    before,
    0,
    []byte{0xa0, 0xa1, 0xa2, 0xa3, 0xa4, 0xa5, 0xa6, 0xa7},
    hddwritecrashcuts.Cut{Kind: hddwritecrashcuts.DuringMediaWrite, WrittenPrefix: &prefix},
)
if err != nil {
    panic(err)
}
fmt.Printf("sector after interruption: %x, persisted prefix: %d\n",
    result.Image[0], result.PersistedPrefixBytes)
```

The before image and request remain caller-owned, read-only inputs for the
duration of `Apply`; the function does not change them. Every returned row is
detached from input and sibling rows.
The fixed-size receipt contains another copy of the acknowledged data. Crash
cuts return no receipt, including a cut after all bytes reach media but before
the response.

Refusals are returned in order as `ErrInvalidImageShape`, `ErrInvalidLBA`,
`ErrInvalidDataLength`, `ErrInvalidCut`, and `ErrInvalidPrefix`. Callers can
test their identity with `errors.Is`. A valid LBA has range `0..31`; a
during-write prefix has range `1..7`; the other cuts require an absent prefix.

## Validation

Run from this module directory with Go 1.26 and `GOTOOLCHAIN=local`:

```sh
sh BUILD
go test -run=^$ -fuzz=FuzzWriteCutHistory -fuzztime=110000x
go test -run=^$ -fuzz=FuzzCrashCutFixture -fuzztime=45000x
GO=/path/to/go TMPDIR=/tmp python3 tools/check_mutations.py
GOARCH=386 go test -exec=/path/to/qemu-i386 ./...
GOOS=windows GOARCH=amd64 go test -c -o /tmp/hdd-write-crash-cuts.test.exe
```

The tests replay the exact shared DB04a vector only after a test-side
byte-array oracle validates every case and complete declared medium. A
separate exhaustive sweep covers all 320 LBA/cut combinations, including
equal-byte interruptions and both LBA boundaries. Fixed-seed compositions
check 5,000 transitions. Fixture fuzzing accepts malformed inputs as expected
refusals and replays only completely valid, model-accepted vectors. The clean
mutation control requires the intended native assertion for each fault.

On Go 1.26.1/Linux/amd64, both `BUILD` and `BUILD_windows` passed with 97.6%
statement coverage, race, and vet. `FuzzWriteCutHistory` passed 110,000 inputs
and `FuzzCrashCutFixture` passed 45,000 inputs. All seven named mutations were
killed by their declared assertions after the clean control. The package tests
executed under Linux/386 QEMU and cross-compiled for Windows/amd64; native
Windows execution was not performed.
