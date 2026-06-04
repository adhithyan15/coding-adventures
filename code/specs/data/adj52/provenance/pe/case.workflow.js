export const meta = {
  name: 'pe-case-adjudication',
  description: 'Phase 2-4: find a real published SUSPECTED-PE case, ingest it blind into the grounded corpus vocabulary (holding ground truth aside), and SEPARATELY run the ungrounded invent-LRs deriver on the same case. The parent then evaluates the observations against the frozen grounded corpus and contrasts grounded vs ungrounded — the proof that byte-provenance is the antidote for hallucination.',
  phases: [{ title: 'Prepare' }, { title: 'Ungrounded' }],
}

// The grounded corpus vocabulary (NOT the LRs — the ingester maps findings to
// these labels; the ungrounded deriver never sees the grounded numbers).
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
    ground_truth: { type: 'string', description: 'the confirmed outcome: did the patient have PE, and how was it established (CTPA, V/Q, autopsy, follow-up)? + correct disposition. Held aside.' },
    pe_confirmed: { type: 'string', enum: ['pe_present', 'pe_excluded', 'uncertain'] },
    prose: { type: 'string', description: 'sanitised case prose up to the diagnostic decision point (no final answer named)' },
    observations: { type: 'array', items: { type: 'string' }, description: 'the subset of the GIVEN vocabulary that is TRUE for this patient, as exact vocabulary strings' },
    observations_excluded_pretest: { type: 'array', items: { type: 'string' }, description: 'observations available BEFORE imaging (everything except the ctpa(...) terms) — for the pretest evaluation' },
    findings_not_in_vocab: { type: 'array', items: { type: 'string' }, description: 'any diagnostically relevant finding the vocabulary cannot express (honesty about corpus coverage)' },
  },
}
const UNGROUNDED_SCHEMA = {
  type: 'object',
  required: ['p_pe', 'invented_lrs', 'answer_text'],
  properties: {
    p_pe: { type: 'number', description: 'the deriver\'s probability of PE' },
    invented_lrs: { type: 'array', items: { type: 'object' }, description: 'each finding -> the LR the deriver assigned, with the citation it attached (if any)' },
    cites_primary_numbers: { type: 'boolean', description: 'did the deriver actually pull sensitivity/specificity/OR numbers from sources, or assign plausible-looking LRs' },
    answer_text: { type: 'string' },
  },
}

const prepPrompt = `Find ONE real published case report of a patient worked up for SUSPECTED pulmonary embolism, where the clinical course describes the standard workup (risk factors / Wells-type features, D-dimer, and CT pulmonary angiography) and the final outcome is DOCUMENTED (PE confirmed, or PE excluded with an alternative explanation, confirmed by imaging / follow-up). Prefer open-access (PMC). Avoid the most famous textbook cases.

1. Use WebSearch/WebFetch to find it. Record source_url.
2. Extract GROUND TRUTH (held aside): did the patient have PE, how was it confirmed/excluded, and the correct disposition. Set pe_confirmed.
3. Produce sanitised prose up to the decision point (do not name the final answer).
4. Map the patient's findings onto EXACTLY this vocabulary (use only these strings; include one only if genuinely TRUE for the patient):
${VOCAB.map((v) => `   - ${v}`).join('\n')}
   Return observations (all true vocab terms) and observations_excluded_pretest (the same list minus any ctpa(...) term — i.e. what is known BEFORE imaging).
5. List any diagnostically relevant finding the vocabulary cannot express (findings_not_in_vocab) — be honest about corpus coverage.`

const ungroundedPrompt = (prose) => `You are a clinical reasoning engine. Read this suspected-PE case and build a small Bayesian rulebook for diagnosis(pulmonary_embolism): assign each relevant finding a prior and a likelihood ratio, then combine them into a probability of PE. Use whatever LR values your judgement suggests; cite sources if you can. Report p_pe (your final probability), invented_lrs (each finding with the LR you assigned and any citation), cites_primary_numbers (did you actually extract sensitivity/specificity/OR figures from real sources, or assign plausible values), and answer_text.

=== CASE ===
${prose}
=== END ===`

const prep = await agent(prepPrompt, { phase: 'Prepare', label: 'prepare:pe-case', agentType: 'general-purpose', schema: PREP_SCHEMA })
const ungrounded = await agent(ungroundedPrompt(prep.prose), { phase: 'Ungrounded', label: 'ungrounded-derive', agentType: 'general-purpose', schema: UNGROUNDED_SCHEMA })

return { prep, ungrounded }
