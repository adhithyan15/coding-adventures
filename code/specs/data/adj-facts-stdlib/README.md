# adj-facts-stdlib — the ADJ standard library of grounded facts (organized by subject)

A growing **standard library of recallable facts** that any ADJ program can `import` and
query. Together with [`adj-formula-stdlib`](../adj-formula-stdlib/) (grounded **formulas and
laws**) and the [medical recall domains](../mycin-2026/recall/), it forms the **ADJ standard
library**: the facts, formulas, and laws of the sciences — chemistry, physics, biology,
mathematics, and beyond — encoded once, provenanced, and reusable.

## Why a standard library of facts (and formulas, and laws)

The goal is that an AI agent working in a domain can **reason through this library** the way a
student reasons up from foundations — e.g. a medicine agent draws on chemistry, physics,
biology, and math the way a medical student builds on them from the start. Recall costs
**zero answer-time model calls**: the engine resolves a binding query against the grounded
rows and returns the answer **with its citation**, on the CPU. Every value is byte-provenanced
from a citable source (see [feedback: nothing human-authored]); nothing is asserted "from
memory," and the trust tier honestly reflects the source (`authoritative` for a primary/official
source — NIST, NASA, IUPAC, PubChem, a standards body; `consensus` for a secondary reference).

## Organized by subject, not by level

Files live under `code/specs/data/adj-facts-stdlib/<subject>/<name>.adj`. There is **no
grade/age categorization** — just subjects. Current subjects (grown one small, grounded library
per rotation, in parallel):

| subject | example library | source |
|---|---|---|
| `geometry/` | polygon → number of sides | Wolfram MathWorld |
| `astronomy/` | planet → order from the Sun | NASA |
| `chemistry/` | element → atomic number | PubChem / NIH |
| `chemistry/` | common substance → approximate pH | LibreTexts (consensus) |
| `chemistry/` | element → periodic-table group family | Wikipedia (consensus) |
| `chemistry/` | common chemical → acid or base | LibreTexts (consensus) |
| `chemistry/` | subatomic particle → electric charge (proton → positive) | DOE "Explains…Nuclei" (authoritative) |
| `chemistry/` | chemical bond type → defining token (ionic → transfer) | LibreTexts (consensus) |
| `chemistry/` | mixture kind → the everyday example the source names (colloid → milk) | LibreTexts (consensus) |
| `metrology/` | SI prefix → power of ten | NIST |
| `mathematics/` | Roman numeral → value | (consensus) |
| `calendar/` | day / month → number | ISO 8601 |
| `money/` | US coin → cents | US Mint |
| `earth-science/` | water-cycle stage → step number | USGS Water Science School |
| `nutrition/` | common food → MyPlate food group | USDA MyPlate |
| `agriculture/` | farm animal → product it gives | Iowa State University (CFSPH) |
| `biology/` | common bone → body region | NIH / MedlinePlus |
| `biology/` | macronutrient → energy (kcal) per gram | NIH / MedlinePlus |
| `biology/` | basic tissue type → representative example | NCI SEER Training |
| `biology/` | kingdom of life → representative example organism (fungi → mushrooms) | Science Notes (consensus) |
| `biology/` | blood-vessel type → defining function (artery → away from heart) | NCI SEER Training |
| `biology/` | hormone → endocrine gland that secretes it | NCI SEER Training / NIH MedlinePlus |
| `biology/` | vitamin → deficiency disease it prevents | NIH Office of Dietary Supplements |
| `biology/` | leaf part → defining token / function (blade → flattened, stomata → gas_exchange) | Colorado State University Extension (authoritative) |
| `physics/` | simple machine → everyday example | NASA |
| `physics/` | phase change → its name (melting, freezing, …) | LibreTexts (consensus) |
| `physics/` | energy form → defining token (chemical → bonds, thermal → heat) | EIA (energy.gov) |
| `physics/` | common force → everyday example NASA uses (gravity → waterfall, tension → ropes) | NASA GSFC Swift (authoritative) |
| `physics/` | named wave → its family (sound → mechanical, radio → electromagnetic) | NASA Science (authoritative) |
| `anatomy/` | lung → number of lobes (right 3, left 2) | NIH / NCI SEER Training |
| `anatomy/` | brain part → primary function it controls | NIH / NCI SEER Training + StatPearls |
| `anatomy/` | skeletal muscle → body region it is located in | Wikipedia (consensus) |
| `anatomy/` | digestive organ → primary function it performs | NIH NIDDK / NCI SEER Training |
| `anatomy/` | synovial joint type → representative example (hinge → elbow) | NIH / NLM StatPearls |
| `anatomy/` | skin layer → defining descriptor (epidermis → outermost, subcutaneous → fat) | NIH / NCI SEER Training |
| `anatomy/` | ear structure → ear region it sits in (cochlea → inner, malleus → middle, ear canal → outer) | NIH NIDCD (authoritative) |
| … | *geography, physical constants, …* | *(expanding)* |

Formulas and laws (Newton's `F = ma`, the ideal gas law `PV = nRT`, area/volume, …) are grown
in `adj-formula-stdlib/<subject>/` using the `formula` construct — simple ones first, growing
more complex — and are consumed the same way.

## Consuming a library

```adj
import "chemistry/elements.adj"
? atomic_number(oxygen, $Z)          % 8, cited to its source
```

Because a `table` row lowers to a relation whose value is a number ([`ADJ-TABLES`](../../ADJ-TABLES.md)),
a recalled fact **composes into a formula** — a looked-up atomic number, side count, or
conversion factor flows straight into arithmetic. That is the bridge from **recall** to
**compute**, and the reason facts and formulas belong in one standard library.
