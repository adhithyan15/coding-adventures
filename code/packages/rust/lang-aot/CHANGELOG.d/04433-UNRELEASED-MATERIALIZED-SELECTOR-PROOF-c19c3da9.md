## [Unreleased]

### Fixed

- Preserve transient materialized conditional-result proof after condition effects and before branch name captures, retaining the original pre-lowering gate and both-arm requirement.
- Keep selector-induced alias refusals and imported computed-name branch acceptance; pair eight name/value controls with exact -1/1 native results. Ordinary value constructors carry no conditional proof.
