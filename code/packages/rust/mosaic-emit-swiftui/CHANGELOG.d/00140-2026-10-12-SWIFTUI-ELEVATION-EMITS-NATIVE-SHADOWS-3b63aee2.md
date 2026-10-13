## 2026-10-12 (SwiftUI elevation emits native shadows)

Supported Mosaic `elevation: raised|overlay` tokens now emit SwiftUI `.shadow`
modifiers with tiered depth and softness. A paired non-inset CSS shadow leaves
the degradation report only when that native depth is emitted; standalone,
inset, and unsupported shadows remain explicit debt (#17365).
