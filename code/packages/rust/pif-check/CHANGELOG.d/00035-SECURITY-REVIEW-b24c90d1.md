## [0.3.1] - 2026-11-24

### Security

- PIF-003c security review: footprints are memoised across claims (`Footprinter`): the dependency graph is condensed (an inductive type, its constructors and recursor are one node), computed bottom-up once per node with an explicit stack and shared between queries, so thousands of claims over one prelude no longer re-walk it per claim. `footprint()` keeps its signature.
