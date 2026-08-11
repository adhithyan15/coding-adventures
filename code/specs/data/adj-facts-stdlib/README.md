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

## Known overlapping tables (discovered 2026-08-11)

Because this library grows via many parallel, independently-scoped rotations, a handful of
tables ended up covering nearly the same ground under different names before anyone
cross-checked the whole tree. Discovered while scoping a new "science 13th slice" candidate:

- `earth-science/rock-types.adj` (`rock_formation(rock_type, forms_from)`, NPS source) and
  `geology/rock-type.adj` (`rock_type(rock, formation_process)`, USGS sources) both table the
  same three rock classes (igneous/sedimentary/metamorphic) and essentially the same "how it
  forms" fact, just with different atom labels and citations. Both are shipped; neither has
  been deprecated. Left as-is pending a maintainer decision on whether to merge/rename.
- `earth-science/water-cycle.adj` (`water_cycle_stage(stage, step_number)`, USGS, 5 rows) was
  discovered to already exist with the SAME predicate name as a candidate "water cycle stages"
  slice that was about to be implemented — that candidate was dropped before writing any code.
- `physics/simple-machines.adj` (`simple_machine_example(machine, example)`, NASA) was
  discovered to already cover the six simple machines' functions in its own header prose before
  a redundant `simple_machine(machine, description)` candidate (sourced from teachengineering.org)
  was implemented — that candidate was also dropped before writing any code.

**Lesson for future rotations**: before scoping a new table, grep the WHOLE
`adj-facts-stdlib/` tree for the candidate's predicate name and topic keywords (not just
`ls` the one subdirectory you plan to write to) — related content easily lands in a
different subject directory than you'd guess (e.g. a "cloud" or "rock" fact could be under
`meteorology/`, `geology/`, or `earth-science/`).

## Organized by subject, not by level

Files live under `code/specs/data/adj-facts-stdlib/<subject>/<name>.adj`. There is **no
grade/age categorization** — just subjects. Current subjects (grown one small, grounded library
per rotation, in parallel):

| subject | example library | source |
|---|---|---|
| `geometry/` | polygon → number of sides | Wolfram MathWorld |
| `geometry/` | angle type → defining measure-condition (acute → between_0_and_90, right → equals_90, reflex → greater_than_180) | Mathematics LibreTexts (consensus) |
| `geometry/` | triangle (by sides) → defining side-condition (equilateral → three_equal_sides, isosceles → two_equal_sides, scalene → three_unequal_sides) | Wolfram MathWorld (authoritative) |
| `geometry/` | quadrilateral family → its one defining property (rectangle → four_right_angles, rhombus → all_sides_same_length, trapezoid → two_sides_parallel) | Wolfram MathWorld (authoritative) |
| `geometry/` | circle part → its defining description (radius → center_to_perimeter, diameter → maximum_distance_across, chord → ends_on_circle) | Wolfram MathWorld (authoritative) |
| `geometry/` | angle-pair relationship → its defining condition (complementary → sum_to_90, supplementary → sum_to_180, vertical → equal, adjacent → share_side_and_vertex) | OpenStax / Mathematics LibreTexts (consensus) |
| `geometry/` | **DERIVED, not looked up** — how many triangles a named quadrilateral decomposes into (`triangle_decomposition_count(shape, 2)`), by a `rule` combining the `quadrilateral_property` table with the general definitions of triangulation and a polygon diagonal — no single source states the composite fact directly | Wolfram MathWorld (consensus; see `shape-composition.adj`'s header for why this earns `consensus` rather than `authoritative`) |
| `astronomy/` | planet → order from the Sun | NASA |
| `astronomy/` | stellar spectral class letter → the color NASA assigns it (o → blue, g → yellow, m → red) | NASA Science (authoritative) |
| `astronomy/` | galaxy type → defining shape NASA states (spiral → spiral_arms, elliptical → round_to_oval, irregular → unusual_shapes) | NASA Science (authoritative) |
| `astronomy/` | celestial-object type → defining property (star → gives_off_light, planet → revolves_around_star, moon → orbits_planet, comet → frozen_gases_and_dust, asteroid → rocky) | NASA StarChild / NASA Science (authoritative) |
| `chemistry/` | element → atomic number | PubChem / NIH |
| `chemistry/` | common substance → approximate pH | LibreTexts (consensus) |
| `chemistry/` | element → periodic-table group family | Wikipedia (consensus) |
| `chemistry/` | common chemical → acid or base | LibreTexts (consensus) |
| `chemistry/` | subatomic particle → electric charge (proton → positive) | DOE "Explains…Nuclei" (authoritative) |
| `chemistry/` | chemical bond type → defining token (ionic → transfer) | LibreTexts (consensus) |
| `chemistry/` | mixture kind → the everyday example the source names (colloid → milk) | LibreTexts (consensus) |
| `chemistry/` | lab equipment → its use (beaker → hold, bunsen_burner → heat) | LibreTexts (consensus) |
| `chemistry/` | measuring tool → the quantity it measures (`measuring_tool(tool, quantity)`, ruler → length, graduated_cylinder → volume, balance → mass, thermometer → temperature) — the "observation and measurement" gap, distinct from the sibling tool→use table above | Chemistry LibreTexts "Introducing Measurements in the Laboratory" (consensus) |
| `chemistry/` | Types of chemical reaction → the defining token each is described by (combination → two_or_more_combine, decomposition → breaks_down, combustion → reacts_with_oxygen). | Chemistry LibreTexts (consensus) |
| `chemistry/` | metal → flame-test color it gives (sodium → orange, potassium → violet, lithium → red) | University of Washington Department of Chemistry (authoritative) |
| `chemistry/` | mixture-separation method → property it separates by (filtration → by_particle_size, distillation → by_volatility, chromatography → by_different_rates) | Chemistry LibreTexts (consensus) |
| `chemistry/` | named gas law → the pair of quantities it relates (boyle → pressure_volume, charles → volume_temperature, avogadro → volume_moles) | Chemistry LibreTexts CK-12 (consensus) |
| `chemistry/` | element category → defining electrical property (metal → good_conductor, nonmetal → poor_conductor, metalloid → semiconductor) | Chemistry LibreTexts (consensus) |
| `metrology/` | SI prefix → power of ten | NIST |
| `mathematics/` | Roman numeral → value | (consensus) |
| `calendar/` | day / month → number | ISO 8601 |
| `money/` | US coin → cents | US Mint |
| `earth-science/` | water-cycle stage → step number | USGS Water Science School |
| `earth-science/` | master soil horizon → what it is (o → organic_matter, c → parent_material, r → bedrock) | UNL passel Plant & Soil Sciences eLibrary (authoritative) |
| `earth-science/` | tectonic plate-boundary type → how plates move (divergent → rip_apart, convergent → subducts, transform → slide_past) | U.S. National Park Service (authoritative) |
| `earth-science/` | atmosphere layer → distinctive feature (troposphere → weather, stratosphere → ozone_layer, thermosphere → auroras) | NASA "Earth's Atmosphere" (authoritative) |
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
| `biology/` | insect body region → what it bears / its function (head → eyes_antennae_and_mouthparts, thorax → legs_and_wings, abdomen → digestion_and_reproduction) | UF/IFAS EDIS Entomology (authoritative) |
| `biology/` | mitosis phase → its defining event (prophase → chromatin_forms_chromosomes, metaphase → chromosomes_line_up, anaphase → chromatids_separate, telophase → nuclear_membrane_forms) | NCI SEER Training (authoritative) |
| `biology/` | mitosis phase → its position in the cycle as a NUMBER (`mitosis_phase_order`, prophase → 1 … telophase → 4), the same source's ordering restated as a bindable fact | NCI SEER Training (authoritative) |
| `biology/` | **DERIVED, not looked up, CROSS-DIRECTORY composition** — each mitotic phase's position in the cycle as an ORDINAL WORD, not just a number (`mitosis_phase_ordinal_position(phase, ordinal)`), by a `rule` bridging the new `mitosis_phase_order` table (`biology/`) to the already-shipped `ordinal_number` table (`mathematics/`) via a relative `../mathematics/ordinal-numbers.adj` import — grounds "anaphase is the THIRD phase of mitosis"; honest abstention on `interphase` (not one of the 4 ordered mitotic phases tabled); the FIRST biology-domain entry in this ordinal-bridge pattern | NCI SEER Training + standard English ordinal-number convention (authoritative/consensus; see `mitosis-phase-ordinal-position.adj`'s header) |
| `biology/` | monarch butterfly life-cycle stage → its position in the cycle as a NUMBER (`monarch_life_stage(stage, order)`, egg → 1, larva → 2, pupa → 3, adult → 4) — the same numbered-cycle shape `earth-science/water-cycle.adj` established, applied to a biological process; honest abstention on `nymph` (the incomplete-metamorphosis term) | USDA Forest Service "Monarch Butterfly Biology" (authoritative) |
| `biology/` | rainforest layer (top to bottom) → a one-fact description of it (`rainforest_layer(layer, description)`, emergent → tallest_trees_dominate_skyline, canopy → deep_treetop_vegetation_layer, understory → dark_humid_layer_below_canopy, forest_floor → darkest_layer_hard_for_plants_to_grow) — picked via the mandatory full-tree-grep-before-scoping check, confirming zero prior coverage | National Geographic Education "Rain Forest" (consensus; see `rainforest-layer.adj`'s header) |
| `biology/` | animal → the biome it lives in (`animal_habitat(animal, biome)`, polar_bear → arctic, bactrian_camel → desert, giraffe → grassland) — a sibling library to the already-shipped `animal-homes.adj` (built structures like hive/nest/burrow), but a genuinely different axis (biome/environment type), confirmed distinct via the mandatory full-tree-grep-before-scoping check | National Geographic (consensus; see `animal-habitat.adj`'s header) |
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
| `physics/` | wave property → what it measures (wavelength → distance_between_identical_parts, frequency → waves_per_second, period → time_for_one_cycle) | OpenStax Physics (consensus) |
| `physics/` | friction type → context it acts in (static → at_rest, sliding → sliding_motion, rolling → spherical_object, fluid → fluid_layers) | Testbook "Types of Friction" (consensus) |
| `physics/` | light behavior → effect on light (reflection → bounces_off, refraction → changes_direction, scattering → variety_of_directions) | NASA Science (authoritative) |
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
| `anatomy/` | respiratory part → function it performs (trachea → main_airway, alveoli → gas_exchange, diaphragm → contracts_inspiration) | NIH / NCI SEER Training (authoritative) |
| `anatomy/` | kidney / urinary part → what it is or does (renal_cortex → outer_region, renal_pelvis → collects_urine, ureter → carries_urine_to_bladder) | NIH NIDDK / NCI SEER Training (authoritative; medulla row Wikipedia consensus) |
| `anatomy/` | heart valve → the two chambers/vessels it separates (tricuspid → right_atrium_and_right_ventricle, aortic → left_ventricle_and_aorta) | NCI SEER Training (authoritative) |
| `anatomy/` | long-bone region → what it is / where it sits (diaphysis → shaft, epiphysis → tip_of_bone, metaphysis → between_diaphysis_and_epiphysis) | NIH NCBI StatPearls (authoritative) |
| `geology/` | Mohs reference mineral → whole-number hardness (talc → 1, quartz → 7, diamond → 10) | NPS "Mohs Hardness Scale" (authoritative) |
| `geology/` | Earth's internal layer → physical state, in the source's own words (crust → rigid, mantle → semi_solid, outer_core → liquid, inner_core → solid) | USGS "This Dynamic Earth" (authoritative) |
| `geology/` | basic rock-type class → how it forms (`rock_type(rock, formation_process)`, igneous → crystallized_molten_rock, sedimentary → deposited_weathered_material, metamorphic → heat_and_pressure_transformation) — a THREE-different-source-page table, unlike its single-source `earth-layers.adj` sibling | USGS "What are igneous/sedimentary/metamorphic rocks?" FAQ pages (authoritative; see `rock-type.adj`'s header) |
| `meteorology/` | Beaufort wind force number → the name the source gives it (0 → calm, 6 → strong_breeze, 12 → hurricane) | NWS Beaufort Wind Scale (authoritative) |
| `meteorology/` | precipitation type → defining physical form (snow → ice_crystals, sleet → frozen_raindrops, hail → balls_of_ice) | NOAA National Weather Service (authoritative) |
| `meteorology/` | Saffir-Simpson hurricane category → the damage descriptor NHC uses (1 → some_damage, 3 → devastating_damage, 5 → catastrophic_damage) | NOAA/NHC Saffir-Simpson Hurricane Wind Scale (authoritative) |
| `meteorology/` | weather instrument → the quantity it measures (`weather_instrument(instrument, quantity)`, anemometer → wind_speed, barometer → atmospheric_pressure, hygrometer → humidity) — a DIFFERENT "observation and measurement" axis from `chemistry/measuring-tools.adj` (lab tools, not weather instruments) | NOAA "Build Your Own Weather Station" (authoritative) |
| `meteorology/` | cloud type → the weather it indicates (`cloud_type(cloud, weather_indication)`, cirrus → approaching_warm_front, cumulonimbus → heavy_rain_thunderstorm, stratus → light_rain_drizzle_or_none) — NOT an instrument-measurement fact like its `weather-instruments.adj`/`ocean-observing-instruments.adj` siblings, but a cloud-appearance-to-weather-forecast fact | NWS Louisville "Cloud Classification" (authoritative; see `cloud-type.adj`'s header) |
| `environment/` | **RANGE table** — AQI value → EPA level of concern, keyed by band minimum (0 → good, 51 → moderate, 101 → unhealthy_for_sensitive_groups, 301 → hazardous, open top band) | EPA AirNow "AQI Basics" (authoritative) |
| `geography/` | common landform → defining descriptor (mountain → projects_above_surroundings, plateau → flat_elevated, canyon → deep_narrow) | USGS Feature Type Thesaurus (authoritative) |
| `geography/` | globe reference line → what it marks (equator → zero_degrees_latitude, prime_meridian → zero_degrees_longitude, tropic_of_cancer → northernmost_sun_overhead) | NOAA National Ocean Service / NESDIS (authoritative) |
| `language/` | Greek letter → its 1-based position in the alphabet (alpha → 1, gamma → 3, pi → 16, omega → 24) | Wikipedia "Greek alphabet" (consensus) |
| `language/` | Latin letter → its International Morse code pattern as dot/dash word-atoms (s → dot_dot_dot, o → dash_dash_dash, e → dot) | ITU-R M.1677-1 via Wikipedia "Morse code" (consensus) |
| `language/` | **DERIVED, not looked up** — which pairs of words RHYME (`rhymes_with(word1, word2)`), by a `rule` combining a `word_family` table (the "-an" family's core CVC members: pan, fan, ran, man, tan, van) with the general word-family principle — no single source states "pan rhymes with fan" directly | Reading Rockets "Meet the Word Families" (consensus; see `word-families.adj`'s header) |
| `language/` | word → how many syllables it has (`syllable_count(word, count)`), as demonstrated by a classroom syllable-segmentation technique (peanut → 2, pencil → 2, sunset → 2, laptop → 2) — a genuinely different literacy sub-skill from rhyme-family derivation, grounding CCSS RF.K.2.b | Reading Rockets "Phonological and Phonemic Awareness: In Practice" (consensus; see `syllable-count.adj`'s header) |
| `language/` | word → its beginning sound / phoneme (`initial_sound(word, sound)`, bell → b, bike → b, boy → b), the site's own canonical phoneme-identity example — a THIRD literacy sub-skill, grounding CCSS RF.K.2.d | Reading Rockets "Reading 101 for Parents: Phonological and Phonemic Awareness" (consensus; see `initial-sound.adj`'s header) |
| `language/` | word → its onset/rime split (`onset_rime(word, onset, rime)`, sleep → sl/eep, blast → bl/ast) — a FOURTH literacy sub-skill, grounding CCSS RF.K.2.c (blend/segment onsets and rimes) | Reading Rockets "Tuning In to the Sounds in Words" (consensus; see `onset-rime.adj`'s header) |
| `language/` | changing one sound in a word to form a new word (`phoneme_substitution(original_word, original_sound, new_sound, new_word)`, make/m → bake/b) — the FIFTH literacy sub-skill, completing CCSS RF.K.2's five named parts, grounding RF.K.2.e | Reading Rockets "Phonological and Phonemic Awareness: In Practice" (consensus; see `phoneme-substitution.adj`'s header) |
| `language/` | compound word → why it makes a good beginner multisyllable-spelling example (`compound_word_spelling_example(word, teaching_use)`, catfish/hotdog/playground/yellowtail → beginner_multisyllable_spelling) — the FIRST literacy slice to move beyond CCSS RF.K.2 into a SPELLING pattern | Reading Rockets "How Spelling Supports Reading" (consensus; see `compound-word-spelling-example.adj`'s header) |
| `language/` | word → its syllable type (`silent_e_word(word, syllable_type)`, wake/whale/while/yoke/yore/rude/hare → vce_long_vowel) — ANOTHER spelling pattern beyond CCSS RF.K.2, the "silent e" / "magic e" (VCe) pattern | Reading Rockets "Six Syllable Types" (consensus; see `silent-e-word.adj`'s header) |
| `language/` | word → its r-controlled vowel digraph (`r_controlled_vowel_word(word, pattern)`, barn/corn/fern/bird/curl → ar/or/er/ir/ur) — a phonics pattern ("bossy r") beyond CCSS RF.K.2 | University of Florida Literacy Institute (authoritative; see `r-controlled-vowel-word.adj`'s header) |
| `language/` | fable → its own narrator-stated moral (`fable_moral(fable, moral)`, tortoise_and_the_hare/shepherds_boy_and_the_wolf/boy_and_the_filberts → their own stated lessons) — the FIRST literacy slice grounding a whole-text comprehension artifact rather than a word-level phonics/spelling fact | George Fyler Townsend's translation of Aesop's Fables via Project Gutenberg (authoritative; see `fable-moral.adj`'s header) |
| `language/` | vocabulary word → its meaning as revealed by a worked context-clue example sentence (`vocabulary_in_context(word, meaning)`, ornithology/frugivorous/inconspicuous → their own cited meanings) | Reading Rockets "Using Context Clues to Understand Word Meanings" (consensus; see `vocabulary-in-context.adj`'s header) |
| `language/` | regular past-tense verb → which of three sounds its -ed ending is pronounced with (`past_tense_ed_sound(word, sound)`, walked/lived/wanted → t_sound/d_sound/id_sound) — a phonics pattern beyond CCSS RF.K.2, distinct from the spelling-pattern (`silent-e-word.adj`, `r-controlled-vowel-word.adj`) and whole-text (`fable-moral.adj`) slices already shipped | 7ESL "Pronunciation of ED: Past Tense Pronunciation for Regular Verbs" (consensus; see `past-tense-ed-sound.adj`'s header) |
| `language/` | regular plural noun → which of three sounds its -s/-es ending is pronounced with (`plural_s_sound(word, sound)`, hats/dogs/boxes → s_sound/z_sound/iz_sound) — a sibling phonics pattern to `past-tense-ed-sound.adj` | Speakspeak "Pronunciation of 's' and 'es' plural endings" (consensus; see `plural-s-sound.adj`'s header) |
| `language/` | common idiom → what it actually means (`idiom_meaning(idiom, meaning)`, piece_of_cake/break_the_ice/under_the_weather → very_easy_to_do/start_a_conversation/feeling_slightly_ill) — a figurative-language skill beyond CCSS RF.K.2, picked via the mandatory full-tree-grep-before-scoping check confirming zero prior coverage | Oxford International English "30 Useful English Idiomatic Expressions & Their Meanings" (consensus; see `idiom-meaning.adj`'s header) |
| `language/` | common word → a synonym of it (`synonym(word, synonym)`, happy/smart/quick → cheerful/bright/fast) — a sibling library to the already-shipped `opposites.adj` (antonyms), same source family and trust tier | English Wiktionary "Synonyms" lines (consensus; see `synonyms.adj`'s header) |
| `physics/` | **DERIVED, not looked up, CROSS-FILE composition** — which phase change heating or cooling CAUSES (`causes_phase_change(direction, name)`), by a `rule` combining a new `heat_direction` table with the ALREADY-SHIPPED sibling `phase_change_name` table — no single source states "heating causes melting" directly, but the general heat-direction principle and the specific change-name mapping are each independently citable | LibreTexts "9.3.1: Melting, Freezing, and Sublimation" (consensus; see `heat-causes-phase-change.adj`'s header — same source `states-of-matter.adj` already cites) |
| `physics/` | **DERIVED, not looked up, CROSS-FILE composition** — which named force CAUSES acceleration of its everyday example (`force_causes_acceleration(force, example)`), by a `rule` combining Newton's second law's own general statement with the ALREADY-SHIPPED sibling `force_example` table — reuses two already-verified NASA citations, zero new sourcing work | NASA Beginner's Guide to Aeronautics (authoritative; see `force-causes-acceleration.adj`'s header — same sources `newton-laws.adj`/`forces.adj` already cite) |
| `earth-science/` | **DERIVED, not looked up, CROSS-DIRECTORY composition** — which numbered month a meteorological season STARTS in (`season_start_month_number(season, number)`), by a `rule` bridging the already-shipped `season_start_month` table (`earth-science/`) to the already-shipped `month_number` table (`calendar/`) via a relative `../calendar/months.adj` import — the first cross-DIRECTORY (not just cross-file, same-directory) `rule` composition in this library; the companion query file lives at the package root so the CLI's import sandbox (rooted at the top-level program's own directory) resolves the `../` hop, mirroring `mathematics/word-problems.adj`'s established cross-directory pattern | NOAA NCEI + ISO 8601 month numbering (authoritative/consensus; see `season-start-month-number.adj`'s header) |
| `astronomy/` | **DERIVED, not looked up, CROSS-DIRECTORY composition** — each planet's position from the Sun as an ORDINAL WORD, not just a number (`planet_ordinal_position(planet, ordinal)`), by a `rule` bridging the already-shipped `planet_order` table (`astronomy/`) to the already-shipped `ordinal_number` table (`mathematics/`) via a relative `../mathematics/ordinal-numbers.adj` import — grounds "Earth is the THIRD planet"; honest abstention on Pluto (not one of the 8 major planets tabled); the companion query file lives at the package root so the CLI's import sandbox resolves the `../` hop, the same pattern `earth-science/season-start-month-number.adj` established | NASA Solar System Planets + standard English ordinal-number convention (authoritative/consensus; see `planet-ordinal-position.adj`'s header) |
| `astronomy/` | **DERIVED, not looked up, CROSS-DIRECTORY composition** — each Moon phase's place in the lunar cycle as an ORDINAL WORD, not just a number (`moon_phase_ordinal_position(phase, ordinal)`), by a `rule` bridging the already-shipped `moon_phase_order` table to the already-shipped `ordinal_number` table (`mathematics/`) — the SAME number-to-ordinal-word bridge pattern `planet-ordinal-position.adj` already established, applied to a second already-shipped `astronomy/` table; honest abstention on "eclipse" (not one of the 8 lunar phases tabled) | NASA Moon Phases + standard English ordinal-number convention (authoritative/consensus; see `moon-phase-ordinal-position.adj`'s header) |
| `music/` | movable-do solfège syllable → its 1-based major-scale degree (do → 1, sol → 5, ti → 7) | Wikipedia "Solfège" (consensus) |
| `oceanography/` | ocean-observing instrument → the quantity it measures or detects (`ocean_instrument(instrument, quantity)`, tide_gauge → sea_level, hydrophone → underwater_sound, sonar → distance_to_object) — a THIRD "observation and measurement" axis after `chemistry/measuring-tools.adj` (lab tools) and `meteorology/weather-instruments.adj` (weather instruments) | NOAA oceanservice.noaa.gov "facts" pages (authoritative; see `ocean-observing-instruments.adj`'s header) |
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

## When no single source states the fact directly: DERIVE it with a `rule`

Every library above is a `table`: one source states the answer, and recall is a plain lookup.
Not every curriculum fact has that shape — `geometry/shape-composition.adj` is the first library
here where no single source states the target conclusion as a citable sentence (checked several
MathWorld pages; a formal mathematics reference does not bother spelling out something this
elementary). Rather than block on a missing verbatim quote, it DERIVES the fact with a `rule`,
composing cited PRIMITIVES:

```adj
rule { head: triangle_decomposition_count($Shape, 2)
       when: quadrilateral_property($Shape, $Property)
       source "…the general definitions this rule's own reasoning is built on…"
       locator "https://mathworld.wolfram.com/…"
       trust consensus }
```

The bar is **human-auditability**, not a verbatim match: a query's `steps` trail names BOTH the
rule's own citation (the general definitions) and the underlying fact's citation (the specific
shape's defining property), so a human can independently check each link in the inference — and
if one is wrong, point at exactly which step and fix it, rather than the whole content family
being unrepresentable. `trust consensus` (not `authoritative`) is the honest tier here: the
CONCLUSION is this library's own derivation from quoted material, not a sentence any single
source states outright.
