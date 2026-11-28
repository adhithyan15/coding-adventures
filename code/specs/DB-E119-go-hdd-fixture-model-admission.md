# DB-E119 — Check Go HDD fixture declarations before runtime replay

The Go HDD fixture fuzzer found a structurally valid modified trace
with inconsistent expectations. The first failing case changed LBA
11 to LBA 1 and still declared the original seek and timing values.
The runtime returned the two ticks expected for LBA 1 from the
initial state. The declared timing for the original LBA 11 was four
ticks. The fuzzer treated the modified declaration as its oracle.
[Go timing profile](DB02b-hdd-simulator-go.md)

Validate every parsed operation and timing probe with the existing
stepped-clock model before replaying the document through the runtime.
Check the expected timing, read bytes, resulting head and clock,
operation refusal and unchanged model state where applicable. Model
boolean and nonrepresentable LBA inputs as typed address refusals.
Use the same admission boundary for the canonical fixture and the
fixture fuzzer. Reject inconsistent declarations before runtime replay.
A canonical admission failure or a runtime mismatch after admission
fails the test.

Replay the saved `f20e13a6d9704837` fuzz input and an isolated LBA
change. Include a consistent edited operation and altered result
bytes, timing and error cases. Check both expected overflow probes and
boundaries ending exactly at the maximum. Then run fixed parser and
command-history fuzz campaigns, native suite, coverage, race, vet and
architecture checks. Keep the shared vector and timing runtime fixed.
[Unicode admission prerequisite](DB-E118-go-hdd-fixture-unicode-scalars.md)

## Local verification

The reviewed adapter SHA-256 is
`3d7a955e4720f2869572f2777255862ee80f72108e852a264a7dc57704abbc51`.
The original saved seed passed by receiving the expected model refusal.
A clean-controlled bypass reproduced the exact stale timing mismatch.
The regression suite also checked a consistent edit, invalid read
bytes, wrong timing components, the wrong typed error and a changed
maximum-edge probe target.

Both guided campaigns completed 100,000 inputs. Native statements
reached 100% coverage with race and vet. Linux 386 ran under QEMU and
Windows amd64 compiled. All 5,987 graph rows and 5,986 Linux recipes
matched the previous export. The runtime and shared vector are
byte-identical to the preflight.
