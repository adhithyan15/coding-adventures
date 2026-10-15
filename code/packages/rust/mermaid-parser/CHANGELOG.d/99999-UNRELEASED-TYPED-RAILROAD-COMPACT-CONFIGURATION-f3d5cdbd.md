## Unreleased - Typed Railroad compact configuration

Railroad `compactMode` accepts case-insensitive true/false through the existing
boolean parser; absence or an empty extracted value defaults to false. Other
present values have an exact typed error. An init directive selects the whole
Railroad object before front matter, so a missing directive field uses its
default rather than merging a front-matter field. Compact state and showMarkers
remain independent. All four dialects retain the complete configuration.

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
