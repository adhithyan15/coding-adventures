## Unreleased — bounded Venn source-style admission

Recognized invalid fill/stroke/text color, nonfinite or negative stroke width,
and opacity outside0..=1 reject the entire parse before target lookup. Unknown
targets and later repairing directives do not rescue invalid recognized values.
Missing properties still retain earlier cascade fields; authored zero remains
explicit. Invalid theme values keep their separate existing fallback behavior.

Color admission uses the existing lexical subset, not a CSS name dictionary,
function-arity/range validator or general renderer-safety guarantee. Existing
source tokens reject percent signs even inside quoted style values; explicit
tests preserve that boundary. Width has no new upper cap. Direct typed IR remains
its existing caller trust boundary; this is a source parser contract.

Actual source `df058580`:957 passed/zero failures/zero ignores across43 targets
in all14 affected packages, with strict all-target warnings/unknown-lints denial.
Tests assert918 exact rejection scenarios,342 complete positive IR cases,
two percent-tokenizer errors,12 original invalid-cascade errors plus12 valid
whole-IR companions, exact external corpus style and208 complete scene/query
cases (192 retained plus16 RGBA alpha/raw-stroke cases). Original invalid JSON
fills remain exact errors; title companions retain complete config/theme oracles.
Independent exact source/focused and terminal reviews passed.

Native execution remains attributed to `f630e6ad`:actual full20 run2765/120 with
one unchanged ignore, compiler884, full unfiltered LANG500 and696 runtime cells
on six available backends. Native166 package inputs differ only in four named
Markdown files;21 mapped external resources remain exact. Static resource maps
include conservative test routes and do not close arbitrary dynamic/toolchain/
OS inputs. Historical Closure36/3705 attribution is not new execution.

Required hosted checks and actual merge remain separate. Final-source Miri
**DEFERRED/PENDING**, never passed; full Closure/visual/provenance completion
remains OPEN.
