export const meta = {
  name: 'pe-stress-three-arm',
  description: 'PE corpus stress test: 3 real published cases spanning the decision space (PE ruled OUT, a high-probability PE confirmed, and a diagnostic trap). Each ingested blind into the PE vocabulary and run through plain Claude (no framework) + the ungrounded invent-LRs deriver. The parent then evaluates each against the frozen grounded corpus. Tests whether the PMC11999957 result holds at n>1.',
  phases: [{ title: 'Prepare' }, { title: 'PlainClaude' }, { title: 'Ungrounded' }],
}

// Three case profiles spanning the decision space (case-blind selection criteria).
const PROFILES = [
  { id: 'ruleout', desc: 'a real published case of suspected PE that was ultimately EXCLUDED / ruled out, with an alternative diagnosis confirmed (e.g. a false-positive D-dimer, low Wells, negative CTPA or negative workup). The point is a true-negative for PE.' },
  { id: 'confirm', desc: 'a real published case of a CONFIRMED high-probability pulmonary embolism (classic risk factors — recent surgery/immobility/malignancy/prior VTE — tachycardia, with PE confirmed on CTPA or V/Q). A clear true-positive.' },
  { id: 'trap', desc: 'a real published case where PE was a DIAGNOSTIC PITFALL — initially missed, atypical, or mimicking another condition — but ultimately confirmed. A hard true-positive.' },
]

const VOCAB = [
  'd_dimer(elevated)', 'd_dimer(normal)',
  'clinical_signs_of_dvt(present)', 'pe_is_leading_diagnosis(present)',
  'heart_rate(over_100)', 'recent_immobilization_or_surgery(present)',
  'previous_vte(present)', 'hemoptysis(present)', 'active_malignancy(present)',
  'ctpa(filling_defect_positive)', 'ctpa(negative)',
]

const PREP_SCHEMA = {
  type: 'object',
  required: ['source_url', 'ground_truth', 'prose', 'observations', 'pe_confirmed'],
  properties: {
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'the confirmed outcome: did the patient have PE, how established, correct disposition. Held aside.' },
    pe_confirmed: { type: 'string', enum: ['pe_present', 'pe_excluded', 'uncertain'] },
    prose: { type: 'string', description: 'sanitised case prose up to the diagnostic decision point (no final answer named)' },
    observations: { type: 'array', items: { type: 'string' }, description: 'subset of the GIVEN vocabulary TRUE for this patient (exact strings)' },
    observations_pretest: { type: 'array', items: { type: 'string' }, description: 'observations available BEFORE imaging (exclude ctpa(...) terms)' },
    findings_not_in_vocab: { type: 'array', items: { type: 'string' }, description: 'relevant findings the vocabulary cannot express' },
  },
}
const PLAIN_SCHEMA = {
  type: 'object',
  required: ['p_pe', 'most_likely_diagnosis', 'next_step', 'answer_text'],
  properties: {
    p_pe: { type: 'number', description: 'plain Claude\'s probability of PE (0-1)' },
    most_likely_diagnosis: { type: 'string' },
    would_image_for_pe: { type: 'boolean', description: 'would it order CTPA / image to chase PE' },
    next_step: { type: 'string' },
    answer_text: { type: 'string' },
  },
}
const UNGROUNDED_SCHEMA = {
  type: 'object',
  required: ['p_pe', 'cites_primary_numbers', 'answer_text'],
  properties: {
    p_pe: { type: 'number' },
    cites_primary_numbers: { type: 'boolean' },
    answer_text: { type: 'string' },
  },
}

const prepPrompt = (p) => `Find ONE real published clinical case for a blinded diagnostic experiment. TARGET PROFILE: ${p.desc}

1. WebSearch/WebFetch to find it (prefer open-access PMC). Record source_url.
2. Extract GROUND TRUTH (held aside): did the patient have PE, how confirmed/excluded, correct disposition. Set pe_confirmed.
3. Sanitise prose up to the decision point (do not name the final answer).
4. Map findings onto EXACTLY this vocabulary (use only these strings, include only if genuinely TRUE):
${VOCAB.map((v) => `   - ${v}`).join('\n')}
   Return observations (all true vocab terms) and observations_pretest (minus any ctpa(...) term).
5. List relevant findings the vocabulary cannot express (findings_not_in_vocab).`

const plainPrompt = (prose) => `You are an experienced physician. A colleague hands you this case. Reason naturally — no template. Do NOT look up any published case. Give: the most likely diagnosis, your probability that this is a pulmonary embolism specifically (p_pe, 0-1), whether you would image for PE (CTPA), your next step, and your reasoning.

=== CASE ===
${prose}
=== END ===`

const ungroundedPrompt = (prose) => `You are a clinical reasoning engine. Build a small Bayesian rulebook for diagnosis(pulmonary_embolism): assign each finding a prior and likelihood ratio from your judgement, combine into a probability. Report p_pe, cites_primary_numbers (did you extract real sensitivity/specificity/OR figures or assign plausible values), and answer_text.

=== CASE ===
${prose}
=== END ===`

const results = await pipeline(
  PROFILES,
  (p) => agent(prepPrompt(p), { phase: 'Prepare', label: `prepare:${p.id}`, agentType: 'general-purpose', schema: PREP_SCHEMA })
    .then((prep) => ({ id: p.id, prep })),
  (o) => agent(plainPrompt(o.prep.prose), { phase: 'PlainClaude', label: `plain:${o.id}`, agentType: 'general-purpose', schema: PLAIN_SCHEMA })
    .then((plain) => ({ ...o, plain })),
  (o) => agent(ungroundedPrompt(o.prep.prose), { phase: 'Ungrounded', label: `ungrounded:${o.id}`, agentType: 'general-purpose', schema: UNGROUNDED_SCHEMA })
    .then((ungrounded) => ({ ...o, ungrounded })),
)
return { per_case: results.filter(Boolean) }
