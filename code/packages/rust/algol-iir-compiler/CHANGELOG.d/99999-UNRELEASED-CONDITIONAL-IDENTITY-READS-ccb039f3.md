## Unreleased — bounded conditional identity reads

Restore the exact narrow shared sign-range helper and keep the dedicated flat
unit proof separate. Admit one conditional front only when both roots are safe.
An exact single-read Real/name identity body evaluates its actual once before
general call invalidation; actual nested calls retain ordinary alias effects.
Exact declaration/signature/RHS-chain guards reject malformed and nonidentity
bodies without forwarding cached proof metadata. Full ALGOL854 and affected
native/compiler suites pass; Miri DEFERRED/PENDING, never passed. General
recursive conditional/range composition remains open.
