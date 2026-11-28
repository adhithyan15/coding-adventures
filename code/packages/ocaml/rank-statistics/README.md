# rank-statistics (OCaml)

Reusable non-mutating sample medians and exact distribution-free confidence
intervals for a population median. The cutoff uses a small base-10^9 integer
helper and needs no Zarith runtime dependency. Midpoints handle subnormal,
normal-boundary, and maximum-finite inputs.

```ocaml
open Coding_adventures_rank_statistics

median [| 1.; 2.; 3.; 4. |]  (* Ok 2.5 *)
median_interval [| 1.; 2.; 3.; 4. |] 95
```

Run `BUILD` (or `BUILD_windows`) from this directory for pinned formatting,
instrumented tests, and an expected-source coverage summary. On the reviewed
Windows OCaml 5.2.1 / Dune 3.17.2 / Bisect 2.8.3 toolchain, the rank library
recorded 122/134 Bisect points (91.04%). Coverage is a separate measurement
from the independent exact sign-enumeration oracle and wide frozen vectors;
these checks do not claim physical clock coverage or a measured native C clock.
The package has no benchmark or Warrant dependency.
