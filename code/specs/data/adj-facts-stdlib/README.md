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
| `geometry/` | angle type → defining measure-condition (acute → between_0_and_90, right → equals_90, reflex → greater_than_180) | Mathematics LibreTexts (consensus) |
| `geometry/` | triangle (by sides) → defining side-condition (equilateral → three_equal_sides, isosceles → two_equal_sides, scalene → three_unequal_sides) | Wolfram MathWorld (authoritative) |
| `astronomy/` | planet → order from the Sun | NASA |
| `astronomy/` | stellar spectral class letter → the color NASA assigns it (o → blue, g → yellow, m → red) | NASA Science (authoritative) |
| `chemistry/` | element → atomic number | PubChem / NIH |
| `chemistry/` | common substance → approximate pH | LibreTexts (consensus) |
| `chemistry/` | element → periodic-table group family | Wikipedia (consensus) |
| `chemistry/` | common chemical → acid or base | LibreTexts (consensus) |
| `chemistry/` | subatomic particle → electric charge (proton → positive) | DOE "Explains…Nuclei" (authoritative) |
| `chemistry/` | chemical bond type → defining token (ionic → transfer) | LibreTexts (consensus) |
| `chemistry/` | mixture kind → the everyday example the source names (colloid → milk) | LibreTexts (consensus) |
| `chemistry/` | lab equipment → its use (beaker → hold, bunsen_burner → heat) | LibreTexts (consensus) |
| `chemistry/` | Types of chemical reaction → the defining token each is described by (combination → two_or_more_combine, decomposition → breaks_down, combustion → reacts_with_oxygen). | Chemistry LibreTexts (consensus) |
| `chemistry/` | metal → flame-test color it gives (sodium → orange, potassium → violet, lithium → red) | University of Washington Department of Chemistry (authoritative) |
| `chemistry/` | mixture-separation method → property it separates by (filtration → by_particle_size, distillation → by_volatility, chromatography → by_different_rates) | Chemistry LibreTexts (consensus) |
| `chemistry/` | named gas law → the pair of quantities it relates (boyle → pressure_volume, charles → volume_temperature, avogadro → volume_moles) | Chemistry LibreTexts CK-12 (consensus) |
| `metrology/` | SI prefix → power of ten | NIST |
| `mathematics/` | Roman numeral → value | (consensus) |
| `calendar/` | day / month → number | ISO 8601 |
| `money/` | US coin → cents | US Mint |
| `earth-science/` | water-cycle stage → step number | USGS Water Science School |
| `earth-science/` | master soil horizon → what it is (o → organic_matter, c → parent_material, r → bedrock) | UNL passel Plant & Soil Sciences eLibrary (authoritative) |
| `earth-science/` | tectonic plate-boundary type → how plates move (divergent → rip_apart, convergent → subducts, transform → slide_past) | U.S. National Park Service (authoritative) |
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
| `biology/` | diet category → food it eats (herbivore → plants, carnivore → animals, omnivore → anything) | U.S. National Park Service (authoritative) |
| `biology/` | flower part → function / role (petal → attract_pollinators, stamen → male, ovary → contains_ovules) | University of Illinois Extension (authoritative) |
| `biology/` | seed part → the role/function the source states (seed_coat → covering, cotyledon → food_storage, embryo → miniature_plant) | USDA Forest Service, Woody Plant Seed Manual, Ch.1 (authoritative) |
| `biology/` | fungus part → defining token / role (hypha → thread_like, mycelium → made_of_hyphae, gills → holds_spores) | UNLV "The Kingdom Fungi" biology faculty page (consensus) |
| `biology/` | vertebrate class → its one distinctive characteristic (bird → feathers, mammal → hair, reptile → dry_scaly_skin, fish → gills) | NPS "Vertebrate Grab Bag" (authoritative) |
| `biology/` | muscle-tissue type → its one distinctive characteristic (skeletal → voluntary, smooth → involuntary, cardiac → intercalated_disks) | NCI SEER Training Modules (authoritative) |
| `biology/` | plant tropism → the stimulus it responds to (phototropism → light, gravitropism → gravity, thigmotropism → touch) | Wikipedia "Tropism" (consensus) |
| `physics/` | simple machine → everyday example | NASA |
| `physics/` | phase change → its name (melting, freezing, …) | LibreTexts (consensus) |
| `physics/` | energy form → defining token (chemical → bonds, thermal → heat) | EIA (energy.gov) |
| `physics/` | common force → everyday example NASA uses (gravity → waterfall, tension → ropes) | NASA GSFC Swift (authoritative) |
| `physics/` | named wave → its family (sound → mechanical, radio → electromagnetic) | NASA Science (authoritative) |
| `physics/` | temperature reference point → the value NIST fixes it at (water_boils_celsius → 100, absolute_zero_kelvin → 0) | NIST (authoritative) |
| `physics/` | magnetic pole pairing → interaction (like_poles → repel, opposite_poles → attract) | NASA Heliophysics Education (authoritative) |
| `physics/` | Additive primary colors of light — red → white, green → white, blue → white (the three RGB primaries that combine to make white light) | HyperPhysics, Georgia State University (authoritative) |
| `physics/` | Basic parts of a simple electric circuit → the role each performs (e.g. `battery` → `provides_dc_power`, `wire` → `carries_current`, `switch` → `opens_or_closes`) | MIT K-12 Maker "Circuit Basics and Components" (consensus) |
| `physics/` | common energy source → whether renewable or nonrenewable (solar → renewable, coal → nonrenewable, nuclear → nonrenewable) | U.S. EIA Energy Explained (authoritative) |
| `physics/` | perceived sound property → physical wave quantity it corresponds to (pitch → frequency, loudness → amplitude, timbre → waveform) | Physics LibreTexts (consensus) |
| `physics/` | optical element → how it acts on parallel light (convex_lens → converges_light, concave_lens → diverges_light, convex_mirror → diverges_light) | OpenStax University Physics / College Physics via Physics LibreTexts (consensus) |
| `physics/` | heat-transfer mode → how it moves heat (conduction → direct_contact, convection → motion_of_gasses_and_liquids, radiation → light_waves) | NASA Next Gen STEM (authoritative) |
| `physics/` | EM-spectrum band → everyday use NASA names (radio → radio_stations, infrared → night_vision, x_ray → teeth) | NASA Imagine the Universe! (authoritative) |
| `physics/` | Newton's law number → short name NASA labels it (1 → inertia, 2 → force, 3 → action_reaction) | NASA Glenn Beginner's Guide (authoritative) |
| `anatomy/` | lung → number of lobes (right 3, left 2) | NIH / NCI SEER Training |
| `anatomy/` | brain part → primary function it controls | NIH / NCI SEER Training + StatPearls |
| `anatomy/` | skeletal muscle → body region it is located in | Wikipedia (consensus) |
| `anatomy/` | digestive organ → primary function it performs | NIH NIDDK / NCI SEER Training |
| `anatomy/` | synovial joint type → representative example (hinge → elbow) | NIH / NLM StatPearls |
| `anatomy/` | skin layer → defining descriptor (epidermis → outermost, subcutaneous → fat) | NIH / NCI SEER Training |
| `anatomy/` | ear structure → ear region it sits in (cochlea → inner, malleus → middle, ear canal → outer) | NIH NIDCD (authoritative) |
| `anatomy/` | hand-bone group → part of the hand it occupies (carpals → base_of_hand, metacarpals → middle_of_hand, phalanges → fingers) | InformedHealth.org / NIH NCBI Bookshelf (consensus) |
| `anatomy/` | foot-bone group → region it occupies (tarsals → heel_and_ankle, metatarsals → midfoot, phalanges → toes) | Wikipedia "Metatarsal bones" (consensus) |
| `anatomy/` | vertebral-column region → vertebra count (cervical → 7, thoracic → 12, lumbar → 5, sacral → 5, coccygeal → 4) | NCBI Bookshelf / StatPearls "Vertebral Column" (consensus) |
| `anatomy/` | eye part → function the source states (cornea → bends_light, retina → turns_light_into_signals, optic_nerve → carries_signals_to_brain) | NIH National Eye Institute (authoritative) |
| `anatomy/` | tooth part → its role/location (enamel → outer_surface, dentin → beneath_enamel, cementum → covers_roots) | MedlinePlus / NIH NCBI Bookshelf (authoritative) |
| `geology/` | Mohs reference mineral → whole-number hardness (talc → 1, quartz → 7, diamond → 10) | NPS "Mohs Hardness Scale" (authoritative) |
| `meteorology/` | Beaufort wind force number → the name the source gives it (0 → calm, 6 → strong_breeze, 12 → hurricane) | NWS Beaufort Wind Scale (authoritative) |
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
