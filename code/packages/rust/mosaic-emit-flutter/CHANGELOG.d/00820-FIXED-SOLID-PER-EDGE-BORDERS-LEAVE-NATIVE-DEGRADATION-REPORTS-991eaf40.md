### Fixed — solid per-edge borders leave native degradation reports (#17098)

`border-{top,right,bottom,left}-style: solid` is now recorded as consumed when
Flutter emits the same positive-width `BorderSide`. Unsupported, zero-width,
and style-only declarations remain explicit degradations.
