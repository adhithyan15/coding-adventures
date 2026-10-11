## [0.426.0] - UNRELEASED

- Preserve ALGOL runtime-real formatting provenance through `sqrt(cos(exp(if condition then nonpositive-unit-sign-root else nonpositive-unit-sign-root)))` when every branch is proven, while rejecting positive, unrestricted, non-runtime-real, and overridden branches.
