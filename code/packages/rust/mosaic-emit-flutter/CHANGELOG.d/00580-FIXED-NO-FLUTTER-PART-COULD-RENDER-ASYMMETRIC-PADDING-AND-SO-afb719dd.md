### Fixed -- no Flutter part could render asymmetric padding, and some rendered none

Two defects with one cause: every padding writer in this emitter took a
**single value**.

- `emit_container` fanned it to four edges with `EdgeInsets.all`.
- The styled-box writer emitted `EdgeInsets.symmetric(horizontal: N)` -- and
  `symmetric` defaults the axis you omit to **zero**, so every box authoring
  vertical padding rendered with none. Engram alone had 48 of these.
- Both guards asked only for the `padding` SHORTHAND, so a part authoring
  nothing but longhands was skipped entirely and got **no padding at all** --
  not collapsed, absent.

Neither could express Trestle's `task-detail`, authored `15 / 16 / 16 / 47`
and rendered `15` on every side.

Padding now resolves per edge the way CSS resolves it -- a longhand wins over
the shorthand, edge by edge, an unmentioned edge is zero -- through one shared
`flutter_padding_edges` used by both writers.

| product | before | after |
| --- | --- | --- |
| Trestle | `all` 73, `symmetric` 3 | `all` 76, `fromLTRB` 12 |
| Engram | `all` 280, `symmetric` 48 | `all` 326, `fromLTRB` 74 |
| VisiCalc | `all` 16, `symmetric` 1 | `all` 17 |
| Venture | `all` 16 | `all` 16 |

`EdgeInsets.symmetric` is gone entirely: 52 emissions that were each dropping
an axis. `fromLTRB` appears 86 times where nothing could before. The totals
also **rise**, because parts that authored only longhands previously emitted
no padding at all.

The emitted values match the source exactly: `task-detail` is now
`fromLTRB(47, 15, 16, 16)`, `content` `fromLTRB(30, 8, 30, 60)`, `topbar`
`fromLTRB(30, 20, 30, 14)`.

`EdgeInsets.all` is still emitted when the four edges agree, so the common
case stays readable.

**State-layer padding is unchanged.** A state that overrides padding still
resolves to one value -- 12 of the 1178 authored padding declarations in the
repo sit inside a state block, and none needs per-edge resolution. That arm
now spreads its value with `all` rather than dropping the vertical axis.

Three tests cover this, and the uniform case is their control so none can
pass by accident. Reverting either writer's arm fails them.

