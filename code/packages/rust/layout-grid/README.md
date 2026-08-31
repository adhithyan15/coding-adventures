# layout-grid

`layout-grid` implements a reusable, host-neutral CSS grid formatting context
over `layout-ir`. It owns track parsing, explicit and implicit placement,
intrinsic/flexible sizing, gaps, and two-axis alignment while delegating child
subtree layout through a callback.

The typed `ext["grid"]` contract keeps computed CSS mapping independent from
the geometry algorithm and is shared by native and web Venture hosts.
