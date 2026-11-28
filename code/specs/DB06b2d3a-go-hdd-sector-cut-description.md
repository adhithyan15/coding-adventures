# DB06b2d3a — Describe a single-sector write cut

A sector write can stop before any bytes land, after a strict
prefix, after all bytes, or after the caller receives its
acknowledgement. A larger disk uses the same eight-byte
sector transition.
[Go HDD write cuts](DB04a1-hdd-write-crash-cuts-go.md) · [HDD geometry](DB06b2d1-go-hdd-geometry.md)

Extend the existing Go package `hdd-write-crash-cuts`.

```go
func Describe(cut Cut) (persistedPrefixBytes int, acknowledged bool, err error)
```

Share the cut admission and byte count with Apply. Describe
checks only the cut descriptor and keeps the documented
precedence. BeforeMediaWrite requires an absent prefix and
returns zero bytes. DuringMediaWrite requires one through
seven persisted bytes and returns that exact prefix.
AfterMediaWriteBeforeResponse requires an absent prefix and
returns eight with acknowledgement false. Acknowledged
requires an absent prefix and returns eight with acknowledgement
true. The other accepted descriptions return acknowledgement
false. An unknown kind returns `ErrInvalidCut`. A rejected
prefix returns `ErrInvalidPrefix`. Refusals return zero and false.

Apply continues to check the full image, relative sector, and
data width before the shared cut admission. Its byte effects,
acknowledgement and result ownership follow the same existing
contract. Describe allocates zero bytes and touches no image.

Cross every cut kind and prefix shape including nil, zero,
one through eight and both native integer limits. Compare the
description with the prior standalone cut oracle and Apply
on both valid and invalid cut inputs. Retain the fixed profile
fixtures and public behavior. Run both build lists, race, vet,
guided inputs and clean controls for accepting zero progress
during a write and treating full media as acknowledged.

## Local verification

The implementation is in `77fdbf77b59`. An independent caller
matched 105 cut descriptions and 16,800 exact whole-image
outcomes, with unchanged caller bytes and zero allocation in
Describe. The original portable fixture, both command lists,
race, vet, tidy, emulated Linux/386 and nine mutation controls
passed. The retained package reached 97.6% statement coverage
with 100% in Describe. Build discovery matched the earlier
package exactly.
