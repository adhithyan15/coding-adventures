### Fixed — qualify same-package component references before composition

Standalone component composition can now receive its owning package identity
and exports. Bare sibling references are qualified before dependency-style
collection and layout inlining, so every backend receives the same resolved
tree.

- Report unsupported native table focus and naming explicitly.

- Diagnose unsupported measured table wheel routing on non-React backends.

