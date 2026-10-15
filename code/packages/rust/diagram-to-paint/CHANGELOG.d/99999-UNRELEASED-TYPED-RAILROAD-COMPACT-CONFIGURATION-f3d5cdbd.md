## Unreleased - Typed Railroad compact configuration

Railroad scenes retain `railroad.compactMode` as normalized true/false metadata.
All 196 previous complete metadata cells keep their eleven original fields and
add the false default key. Forty compacttrue controls retain bounded primitive,
style, glyph, path, marker, order and actual font-query expectations; some
geometry expectations use actual layout fields. They are not independent
absolute-layout goldens or direct whole-scene mode-equivalence tests. Separately,
layout and geometry production remains byte-exact; no tighter geometry is promised.

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
