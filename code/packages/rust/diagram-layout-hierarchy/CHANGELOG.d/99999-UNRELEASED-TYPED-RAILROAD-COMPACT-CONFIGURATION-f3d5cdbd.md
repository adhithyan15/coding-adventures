## Unreleased - Typed Railroad compact configuration

Railroad layout retains `compact_mode` with the complete typed configuration.
The flag introduces no layout or geometry branch; all existing numeric, font,
spacing, arc and derived-geometry checks remain. Configuration preservation is
separate from the still-open visual and resource limits documented below.

Integrate the producer bool/default/parser/metadata additions with no further
production repair. Preserve all numeric, effective-font-cap, lexical fallback,
arc, alias and resource controls. Parser controls verify96 complete accepted
configs,84 exact errors and12 directive precedence cases; focused parser25,
layout14 and paint7 pass. Full fourteen-package style/downstream suites pass
894/0/0 over38 targets, plus strict lint. Native223f1789/78+strict is carried
only by complete canonical166-package input identity with four named prior
documentation exceptions, not new execution. Hosted/native checks and visual
limits remain separate. Miri DEFERRED/PENDING, never passed; full Closure parity
and output/source/transformation lineage remain open.
