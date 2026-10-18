## Unreleased — owned Timeline font families

Timeline configuration now carries a validated font family from the owned
themeVariables boundary into every Timeline font query and effective metadata.
The default is the opaque family string `"trebuchet ms", verdana, arial, sans-serif`;
it is one query member, without CSS fallback-list parsing. Leading/trailing
whitespace is trimmed; disallowed interior characters fall back to that default.
Title/label roles, sizes, weights, italic/stretch, geometry and source ownership
remain unchanged. None-config non-Timeline callers retain their supplied families.
At `0b7687390d942b02853dfb04a778cc73b8d3d302`, all 1174 native tests passed across 57 harnesses
(0 ignored), with 2 doctests passed/0 ignored across
14 libraries and all 14 strict all-target checks passing. New complete controls
cover 84 source IR cases, 48 source products, nine direct typed-family cases and
two None-config title/label cases, alongside all retained numeric/palette controls.
This is bounded implementation progress; PR17288 remains unmerged with current
conflict, hosted checks, review and actual-merge reachability gates outstanding.
Font size/layout expansion and native font rendering acceptance remain open.
Miri DEFERRED/PENDING, never passed; full Closure parity and lineage remain OPEN.
