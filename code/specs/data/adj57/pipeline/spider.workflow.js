export const meta = {
  name: 'adj66-spider-ground-weights',
  description: 'ADJ66 — the spider. ADJ65 found the decision rests on ASSUMED (ungrounded) weights. The principle: nothing may be asserted that is not grounded to bytes — in the input OR in the rulebook we derive for it. The weights are rulebook claims, so the spider FETCHES a source for each load-bearing weight (web search/fetch), grounds the weight in a VERBATIM passage from that source (the rulebook byte), and records the citation. Where no source supports a weight it is flagged ungrounded (not laundered). Then the decision is re-run on grounded weights to see whether it shifts off the overconfident wrong answer.',
  phases: [{ title: 'GroundWeights' }],
}

const CASE_TEXT = `A forty-year Indian traveled through Uganda, Tanzania and Kenya. He developed a swelling over the left ankle after an insect bite, then high grade fever with chills and rigors. Hepatomegaly 3 cm, splenomegaly 3 cm. Malaria smear negative. Blood/urine cultures sterile. Widal normal. Viral markers negative. CSF acellular, normal glucose, raised protein. Bone biopsy: single ill-defined granuloma. Brain MRI: mild meningeal enhancement; small pontine T2/FLAIR lesions.`

const HYPOTHESES = [
  'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)',
  'Malaria',
  'Enteric (typhoid) fever',
  'Visceral leishmaniasis (kala-azar)',
  'Tuberculosis (disseminated / CNS)',
  'Brucellosis',
  'Arboviral / other viral encephalitis',
]

// the current model's weights (from the ADJ65 run) — most tagged "assumed".
const CURRENT_EVIDENCE = [
  { name: 'travel_to_east_africa', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 12, Malaria: 6, 'Enteric (typhoid) fever': 2, 'Visceral leishmaniasis (kala-azar)': 5, 'Tuberculosis (disseminated / CNS)': 1, Brucellosis: 3, 'Arboviral / other viral encephalitis': 4 } },
  { name: 'ankle_swelling_after_insect_bite', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 13, Malaria: 1, 'Enteric (typhoid) fever': -3, 'Visceral leishmaniasis (kala-azar)': 4, 'Tuberculosis (disseminated / CNS)': -2, Brucellosis: -2, 'Arboviral / other viral encephalitis': 3 } },
  { name: 'high_grade_fever_with_chills_rigors', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 4, Malaria: 6, 'Enteric (typhoid) fever': 4, 'Visceral leishmaniasis (kala-azar)': 4, 'Tuberculosis (disseminated / CNS)': 2, Brucellosis: 4, 'Arboviral / other viral encephalitis': 2 } },
  { name: 'partial_response_to_empirical_antibiotics', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 2, Malaria: 1, 'Enteric (typhoid) fever': -2, 'Visceral leishmaniasis (kala-azar)': 2, 'Tuberculosis (disseminated / CNS)': 1, Brucellosis: -1, 'Arboviral / other viral encephalitis': 2 } },
  { name: 'hepatosplenomegaly', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 5, Malaria: 4, 'Enteric (typhoid) fever': 4, 'Visceral leishmaniasis (kala-azar)': 8, 'Tuberculosis (disseminated / CNS)': 3, Brucellosis: 4, 'Arboviral / other viral encephalitis': -1 } },
  { name: 'malaria_smear_negative', source: 'grounded', citation: 'thick-film sensitivity ~90-95%; one negative ~10x down', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 1, Malaria: -10, 'Enteric (typhoid) fever': 1, 'Visceral leishmaniasis (kala-azar)': 1, 'Tuberculosis (disseminated / CNS)': 1, Brucellosis: 1, 'Arboviral / other viral encephalitis': 1 } },
  { name: 'blood_urine_cultures_sterile', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 2, Malaria: 1, 'Enteric (typhoid) fever': -6, 'Visceral leishmaniasis (kala-azar)': 2, 'Tuberculosis (disseminated / CNS)': 1, Brucellosis: -3, 'Arboviral / other viral encephalitis': 2 } },
  { name: 'widal_negative', source: 'grounded', citation: 'Widal sens ~60-80%; negative modestly lowers typhoid', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 1, Malaria: 0, 'Enteric (typhoid) fever': -5, 'Visceral leishmaniasis (kala-azar)': 1, 'Tuberculosis (disseminated / CNS)': 0, Brucellosis: 0, 'Arboviral / other viral encephalitis': 1 } },
  { name: 'viral_markers_negative', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 2, Malaria: 1, 'Enteric (typhoid) fever': 1, 'Visceral leishmaniasis (kala-azar)': 2, 'Tuberculosis (disseminated / CNS)': 1, Brucellosis: 1, 'Arboviral / other viral encephalitis': -7 } },
  { name: 'csf_acellular_normal_glucose_raised_protein', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 6, Malaria: -2, 'Enteric (typhoid) fever': -1, 'Visceral leishmaniasis (kala-azar)': 0, 'Tuberculosis (disseminated / CNS)': -2, Brucellosis: 1, 'Arboviral / other viral encephalitis': -3 } },
  { name: 'bone_granuloma', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 1, Malaria: -2, 'Enteric (typhoid) fever': 0, 'Visceral leishmaniasis (kala-azar)': 4, 'Tuberculosis (disseminated / CNS)': 7, Brucellosis: 5, 'Arboviral / other viral encephalitis': -3 } },
  { name: 'pontine_t2_flair_lesions_meningeal_enhancement', source: 'assumed', weights: { 'East African trypanosomiasis (T. b. rhodesiense, meningoencephalitic stage)': 9, Malaria: 0, 'Enteric (typhoid) fever': -1, 'Visceral leishmaniasis (kala-azar)': -1, 'Tuberculosis (disseminated / CNS)': 5, Brucellosis: 3, 'Arboviral / other viral encephalitis': 5 } },
]

// the load-bearing discriminating facts ADJ65 flagged — ground these first.
const FACTS_TO_GROUND = [
  'ankle_swelling_after_insect_bite',
  'travel_to_east_africa',
  'pontine_t2_flair_lesions_meningeal_enhancement',
  'csf_acellular_normal_glucose_raised_protein',
  'bone_granuloma',
  'hepatosplenomegaly',
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['fact', 'weights', 'citations', 'ungrounded_hypotheses'],
  properties: {
    fact: { type: 'string' },
    weights: { type: 'object', additionalProperties: { type: 'number' }, description: 'the GROUNDED weight of evidence (decibans) for this fact toward EACH hypothesis, derived ONLY from what the cited sources support.' },
    citations: {
      type: 'array',
      description: 'one or more real sources you fetched. Each grounds a weight in a VERBATIM passage.',
      items: {
        type: 'object', required: ['hypothesis', 'url', 'quote', 'derived_weight_db', 'rationale'],
        properties: {
          hypothesis: { type: 'string' },
          url: { type: 'string', description: 'the source URL you fetched' },
          quote: { type: 'string', description: 'a VERBATIM passage copied from that source that supports the weight' },
          derived_weight_db: { type: 'number', description: 'the decibans this passage justifies for fact->hypothesis' },
          rationale: { type: 'string', description: 'how the passage maps to the weight' },
        },
      },
    },
    ungrounded_hypotheses: { type: 'array', items: { type: 'string' }, description: 'hypotheses for which you could NOT find a supporting source — their weight is set conservatively toward 0 and flagged, NOT invented.' },
  },
}

const groundPrompt = (fact) => `You are a spider grounding ONE rulebook weight in real source bytes. The principle: a weight (how strongly a finding argues for a diagnosis) may NOT be asserted unless it is grounded in a cited source passage — never your own recollection.

CASE CONTEXT (for relevance only):
${CASE_TEXT}

FACT TO GROUND: "${fact}"
COMPETING HYPOTHESES:
${HYPOTHESES.map((h) => `  - ${h}`).join('\n')}

Use web search and web fetch to find authoritative clinical sources (reviews, StatPearls/NCBI, WHO, journal articles) on how "${fact}" relates to EACH hypothesis. For each hypothesis you can support:
  - cite the source URL and copy a VERBATIM passage (quote) that establishes the association (e.g. "a trypanosomal chancre develops at the tsetse-fly bite site"; "neurobrucellosis CSF shows raised protein with lymphocytic or acellular picture");
  - derive a weight of evidence in DECIBANS (10*log10 LR): strongly characteristic ~ +10 to +15, supportive ~ +3 to +8, neutral ~ 0, argues against ~ negative. Map the passage to the number in rationale.
Set weights ONLY from what the sources support. For any hypothesis you cannot find a source for, set its weight to 0 and list it in ungrounded_hypotheses (do NOT invent a number). Be honest: if a fact is non-specific, its weights should be small. Return weights for ALL ${HYPOTHESES.length} hypotheses (the cited ones grounded, the rest 0 + flagged).`

// ---- run: ground each discriminating fact from real sources, in parallel ----
const grounded = await parallel(FACTS_TO_GROUND.map((f) => () =>
  agent(groundPrompt(f), { phase: 'GroundWeights', label: `ground:${f.slice(0, 22)}`, agentType: 'general-purpose', schema: GROUND_SCHEMA })))

// the grounding agents sometimes decorate the fact name (append a parenthetical, change
// case); match on the leading token, case-insensitively, back to the canonical name.
const norm = (n) => (n || '').split(/[\s(]/)[0].trim().toLowerCase()
const groundedByName = new Map(grounded.filter(Boolean).map((g) => [norm(g.fact), g]))

// rebuild the evidence array: replace grounded facts' weights; keep the rest.
const regrounded = CURRENT_EVIDENCE.map((e) => {
  const g = groundedByName.get(norm(e.name))
  if (!g) return e
  return {
    name: e.name,
    source: 'grounded',
    weights: g.weights,
    citations: g.citations,
    ungrounded_hypotheses: g.ungrounded_hypotheses,
  }
})

return {
  case_text: CASE_TEXT,
  hypotheses: HYPOTHESES,
  facts_grounded: [...groundedByName.keys()],
  evidence: regrounded,
  rulebook: grounded.filter(Boolean).flatMap((g) => (g.citations || []).map((c) => ({ fact: g.fact, ...c }))),
  ground_truth: 'Neurobrucellosis (confirmed by Brucella serology + culture, held aside).',
}
