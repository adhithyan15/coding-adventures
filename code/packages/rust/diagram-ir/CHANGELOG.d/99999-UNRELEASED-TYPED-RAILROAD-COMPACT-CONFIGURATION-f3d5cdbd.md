## Unreleased - Typed Railroad compact configuration

`RailroadConfig.compact_mode` preserves authored `compactMode` as a bool,
defaulting to false. Layout retains that configuration and PaintScene exports
its normalized string in `railroad.compactMode`. This is configuration transport;
it introduces no compact-specific layout or geometry branch.

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
