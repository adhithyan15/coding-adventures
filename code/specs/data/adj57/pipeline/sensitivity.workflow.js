export const meta = {
  name: 'adj65-sensitivity',
  description: 'ADJ65 — uncertainty as a first-class primitive. Given a case and its grounded facts, the model proposes the competing hypotheses and a weight-of-evidence matrix (each fact contributes decibans toward each hypothesis, tagged grounded=cites a real likelihood ratio, or assumed=an estimate). The deterministic sensitivity engine then computes the decision, its margin, the load-bearing evidence, what would flip it, and — the honest part — whether the margin rests on ungrounded weights. Demonstrates "if we make some probability shift, how would the decision shift" on the neurobrucellosis case.',
  phases: [{ title: 'WeighEvidence' }],
}

// The neurobrucellosis case + its grounded facts (the discriminating subset).
const CASE_TEXT = `A forty-year Indian traveled through Uganda, Tanzania and Kenya. He developed a swelling over the left ankle after an insect bite, then high grade fever with chills and rigors after 3-4 days. He took empirical antibiotics with only minor relief. He was febrile (102 F). Abdomen revealed hepatomegaly of 3 cm and splenomegaly of 3 cm. Blood smear for malaria parasite was negative. Cultures from blood and urine were sterile. Widal test showed normal titers. Testing for viral markers were negative. CSF was acellular with normal glucose; the proteins were raised. Bone biopsy showed a single ill defined granuloma. MRI of brain showed mild meningeal enhancement; few small lesions in pons, hyperintense on T2/FLAIR.`
const FACTS = [
  'travel_to_east_africa',
  'ankle_swelling_after_insect_bite',
  'high_grade_fever_with_chills_rigors',
  'partial_response_to_empirical_antibiotics',
  'hepatosplenomegaly',
  'malaria_smear_negative',
  'blood_urine_cultures_sterile',
  'widal_negative',
  'viral_markers_negative',
  'csf_acellular_normal_glucose_raised_protein',
  'bone_granuloma',
  'pontine_t2_flair_lesions_meningeal_enhancement',
]

const MODEL_SCHEMA = {
  type: 'object',
  required: ['hypotheses', 'evidence'],
  properties: {
    hypotheses: {
      type: 'array', items: { type: 'string' },
      description: 'the competing diagnoses a careful clinician would weigh for this presentation (4-7).',
    },
    evidence: {
      type: 'array',
      description: 'one entry per fact; its weight of evidence toward EACH hypothesis.',
      items: {
        type: 'object', required: ['name', 'weights', 'source', 'citation'],
        properties: {
          name: { type: 'string', description: 'the fact name' },
          weights: {
            type: 'object',
            description: 'hypothesis -> weight in DECIBANS (10*log10 likelihood ratio). +10 = the fact makes this hypothesis 10x more likely; -10 = 10x less; 0 = no information. Use the SAME hypothesis keys as `hypotheses`.',
            additionalProperties: { type: 'number' },
          },
          source: { type: 'string', enum: ['grounded', 'assumed'], description: 'grounded = you are citing a published/derivable likelihood ratio for this fact->hypothesis link; assumed = your own estimate with no specific source.' },
          citation: { type: 'string', description: 'if grounded: the source of the likelihood ratio. if assumed: "".' },
        },
      },
    },
  },
}

const weighPrompt = `Build a weight-of-evidence model for this case so a decision engine can compute the diagnosis AND its sensitivity.

CASE:
${CASE_TEXT}

GROUNDED FACTS (each is already byte-justified; weigh each one):
${FACTS.map((f) => `  - ${f}`).join('\n')}

1. List the competing HYPOTHESES (diagnoses) a careful clinician would weigh — 4 to 7 of them.
2. For EACH fact, give its weight of evidence toward EACH hypothesis, in DECIBANS (10 * log10 of the likelihood ratio): +10 means the fact makes that hypothesis ~10x more likely, -10 means ~10x less, 0 means uninformative. A fact can support several hypotheses and argue against others (e.g. a negative malaria smear is strongly negative for malaria, ~neutral for the rest).
3. Tag each fact's weighting honestly: source="grounded" ONLY if you are citing an actual published or derivable likelihood ratio (give the source in citation); otherwise source="assumed" with citation="". Be honest — most clinical LRs for a presentation like this are not precisely published, so most weights will be "assumed". That is the point: the engine will report how much the decision rests on those assumed weights.

Do NOT pick the answer yourself; just supply the hypotheses and the weight matrix. The engine decides.`

// ---- run ----
const model = await agent(weighPrompt, { phase: 'WeighEvidence', label: 'weigh-evidence', agentType: 'general-purpose', schema: MODEL_SCHEMA })

return {
  case_text: CASE_TEXT,
  facts: FACTS,
  hypotheses: model.hypotheses,
  evidence: model.evidence,
  ground_truth: 'Neurobrucellosis (confirmed by Brucella serology + culture, held aside from the weighting).',
}
