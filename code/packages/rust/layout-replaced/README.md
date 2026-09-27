# layout-replaced

Reusable intrinsic sizing and object-fit geometry for replaced boxes in the
shared Layout IR. Producers supply decoded intrinsic dimensions and optional
preferred aspect ratios through `ext["replaced"]`; layout, paint, and host
toolkits consume the same finite geometry.

Spec: [`UI45-layout-replaced`](../../../specs/UI45-layout-replaced.md).
