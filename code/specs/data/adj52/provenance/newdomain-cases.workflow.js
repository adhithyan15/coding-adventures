export const meta = {
  name: 'newdomain-three-arm-cases',
  description: 'Diagnose one real case in each NEW grounded domain (strep pharyngitis, bacterial meningitis) through all three arms: ingest blind into the domain vocabulary, plain Claude (no framework), and the ungrounded invent-LRs deriver. The parent runs the grounded-corpus arm deterministically and builds the cross-domain scorecard.',
  phases: [{ title: 'Prepare' }, { title: 'PlainClaude' }, { title: 'Ungrounded' }],
}

const DOMAINS = [
  {
    id: 'strep',
    target: 'group A streptococcal pharyngitis',
    find: 'a real published or well-documented case of a patient (state whether child or adult) presenting with acute pharyngitis / sore throat, where the workup (Centor features, rapid antigen test and/or throat culture) and the final GAS status (streptococcal vs viral) are documented',
    vocab: ['tonsillar_exudate(present)', 'tender_anterior_cervical_nodes(present)', 'history_of_fever(present)', 'cough(absent)', 'age(under_15)', 'rapid_antigen_test(positive)', 'rapid_antigen_test(negative)', 'throat_culture(positive)'],
    pretest_exclude_prefixes: ['rapid_antigen_test(', 'throat_culture('],
    target_question: 'group A strep pharyngitis (GAS)',
  },
  {
    id: 'meningitis',
    target: 'bacterial meningitis',
    find: 'a real published case of a patient with suspected acute meningitis who underwent lumbar puncture, where the CSF results and the final diagnosis (bacterial vs viral/aseptic meningitis) are documented',
    vocab: ['csf_gram_stain(positive)', 'csf_neutrophilic_pleocytosis(high)', 'csf_glucose(low)', 'csf_protein(elevated)', 'csf_lactate(elevated)', 'serum_procalcitonin(elevated)', 'seizure(present)', 'csf_culture(positive)'],
    pretest_exclude_prefixes: ['csf_culture('],
    target_question: 'bacterial (vs viral) meningitis',
  },
]

const PREP_SCHEMA = {
  type: 'object',
  required: ['source_url', 'ground_truth', 'target_present', 'prose', 'observations'],
  properties: {
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'final confirmed diagnosis + how established + disposition. Held aside.' },
    target_present: { type: 'string', enum: ['present', 'excluded', 'uncertain'], description: 'was the TARGET diagnosis present' },
    prose: { type: 'string', description: 'sanitised case prose to the decision point (no final answer named)' },
    observations: { type: 'array', items: { type: 'string' }, description: 'subset of the GIVEN vocabulary TRUE for this patient (exact strings)' },
    observations_pretest: { type: 'array', items: { type: 'string' }, description: 'observations available before the confirmatory test (exclude the listed test prefixes)' },
    findings_not_in_vocab: { type: 'array', items: { type: 'string' } },
  },
}
const PLAIN_SCHEMA = {
  type: 'object',
  required: ['p_target', 'most_likely_diagnosis', 'next_step', 'answer_text'],
  properties: {
    p_target: { type: 'number', description: 'probability of the target diagnosis (0-1)' },
    most_likely_diagnosis: { type: 'string' },
    would_treat_for_target: { type: 'boolean', description: 'would it treat/act for the target diagnosis now' },
    next_step: { type: 'string' },
    answer_text: { type: 'string' },
  },
}
const UNGROUNDED_SCHEMA = {
  type: 'object',
  required: ['p_target', 'cites_primary_numbers', 'answer_text'],
  properties: { p_target: { type: 'number' }, cites_primary_numbers: { type: 'boolean' }, answer_text: { type: 'string' } },
}

const prepPrompt = (d) => `Find ONE real case for a blinded diagnostic experiment. TARGET DOMAIN: ${d.target}. FIND: ${d.find}.

1. WebSearch/WebFetch (prefer open-access). Record source_url.
2. GROUND TRUTH (held aside): final diagnosis, how established, disposition. Set target_present (was it ${d.target}).
3. Sanitise prose to the decision point (do not name the final answer).
4. Map findings onto EXACTLY this vocabulary (only these strings, include only if genuinely TRUE):
${d.vocab.map((v) => `   - ${v}`).join('\n')}
   Return observations (all true vocab terms) and observations_pretest (excluding terms starting with: ${d.pretest_exclude_prefixes.join(', ')}).
5. findings_not_in_vocab: relevant findings the vocabulary cannot express.`

const plainPrompt = (d, prose) => `You are an experienced physician. A colleague hands you this case. Reason naturally — no template. Do NOT look up any published case. Give: most likely diagnosis, your probability that this is ${d.target_question} specifically (p_target, 0-1), whether you would treat/act for it now, your next step, and reasoning.

=== CASE ===
${prose}
=== END ===`

const ungroundedPrompt = (d, prose) => `You are a clinical reasoning engine. Build a small Bayesian rulebook for diagnosis(${d.id === 'strep' ? 'group_a_strep_pharyngitis' : 'bacterial_meningitis'}): assign each finding a prior and likelihood ratio from your judgement, combine into a probability. Report p_target, cites_primary_numbers (did you extract real sensitivity/specificity/OR figures, or assign plausible values), and answer_text.

=== CASE ===
${prose}
=== END ===`

const results = await pipeline(
  DOMAINS,
  (d) => agent(prepPrompt(d), { phase: 'Prepare', label: `prepare:${d.id}`, agentType: 'general-purpose', schema: PREP_SCHEMA }).then((prep) => ({ d, prep })),
  (o) => agent(plainPrompt(o.d, o.prep.prose), { phase: 'PlainClaude', label: `plain:${o.d.id}`, agentType: 'general-purpose', schema: PLAIN_SCHEMA }).then((plain) => ({ ...o, plain })),
  (o) => agent(ungroundedPrompt(o.d, o.prep.prose), { phase: 'Ungrounded', label: `ungrounded:${o.d.id}`, agentType: 'general-purpose', schema: UNGROUNDED_SCHEMA }).then((ungrounded) => ({ domain: o.d.id, target: o.d.target, prep: o.prep, plain: o.plain, ungrounded })),
)
return { per_domain: results.filter(Boolean) }
