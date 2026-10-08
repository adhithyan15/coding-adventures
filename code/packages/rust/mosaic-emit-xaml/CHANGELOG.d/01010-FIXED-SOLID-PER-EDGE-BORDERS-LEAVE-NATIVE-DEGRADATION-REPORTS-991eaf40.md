### Fixed — solid per-edge borders leave native degradation reports (#17098)

`border-{top,right,bottom,left}-style: solid` is no longer reported as
dropped when XAML actually lowers the same positive-width edge. Unsupported,
zero-width, and style-only declarations remain explicit degradations.
