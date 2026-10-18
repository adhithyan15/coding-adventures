## Unreleased — Timeline size ingress and checked admission

TimelineConfig now carries font_size with a16.0 default. Only direct fontSize
in the first closed init root, with absent-member fallback to config.fontSize
YAML, supplies the value. Existing decoding/unit scales and ownership remain;
invalid source values fall back to16.0. Checked Timeline layout rejects invalid
typed sizes after the established canvas/padding/margin priorities, using the
new appended TimelineFontSize error. Every positive finite size, including
subnormal/MAX, is transported without changing the existing layout formulas.
This is ingress/admission only. Caller paint sizes, glyphs, metadata, font roles,
queries and current family transport remain exact; size-driven typography and
wrapping still require their own bounded contract and represented-operation tests.
At `a3172c6569863bad3c02075f3090443c94f6c8cb`, 1181 native tests passed across59 unfiltered harnesses
(0 ignored); 2 doctests passed/0 ignored across14
libraries; all14 strict all-target checks passed. New complete controls cover
132source IR cells,84positive/empty typed layouts,12invalid inputs,8priority
cases and compatibility panic, retaining all old complete Scene/query controls.
Public API migration:56 mandatory literal fields+2field/default declarations
and1appended error variant, with five true struct updates unchanged. The retained
custom root24 source changes only12 expected semantic/layout config values.
PR17288 remains unmerged with current-main/conflict/review/hosted/actual-merge
gates outstanding. Miri DEFERRED/PENDING, never passed; full Closure parity,
transformation lineage and output-byte/source mapping remain OPEN.
