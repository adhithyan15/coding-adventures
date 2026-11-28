# hdd-geometry

`hdd-geometry` maps a signed relative block number to zero-based cylinder,
surface, and sector coordinates, and maps valid coordinates back. It supports
only geometries whose eight-byte block count is within the portable exact
integer bound. Product bounds are checked by division before multiplication.

The package has no media, timing, allocation, or durability policy. It leaves
the fixed DB02 HDD profile and its public simulator unchanged.

Run `sh BUILD` on Unix. `BUILD_windows` lists the direct Go commands for Windows.
