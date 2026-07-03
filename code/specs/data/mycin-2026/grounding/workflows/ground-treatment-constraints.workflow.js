// ground-treatment-constraints.workflow.js — spider-ground the contraindication /
// interaction RULES the constraint optimizer uses (CC-3 of CHART-AS-CONSTRAINTS.md).
//
// CC-1/CC-2 turn chart facts into exclusions + dose-feasibility constraints, but the
// underlying clinical RULES (which allergy excludes which class, which drug is unsafe in
// pregnancy, which combination is additively nephrotoxic) are still structural. CC-3
// grounds those rules through the same cold path — byte-provenanced against FDA labels /
// IDSA / ACOG, adversarially re-extracted — so the constraints rest on grounded facts, not
// authored ones. Output → grounding/treatment-constraints-grounding.json.

export const meta = {
  name: 'ground-treatment-constraints',
  description: 'Spider-ground antibiotic contraindication + interaction rules (allergy cross-reactivity, pregnancy, additive nephrotoxicity, QT) against primary sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per rule: WebSearch/WebFetch a primary source, verbatim byte-quote + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'ci_penicillin_cephalosporin', kind: 'contraindication',
    claim: 'In a patient with a penicillin allergy, what is the cross-reactivity risk with cephalosporins (quantify the rate, esp. 3rd/4th-gen)?' },
  { id: 'ci_aztreonam_safe_penicillin', kind: 'safe_alternative',
    claim: 'Aztreonam (a monobactam) does NOT cross-react with penicillins and is safe to use in patients with a severe penicillin/beta-lactam allergy.' },
  { id: 'ci_moxifloxacin_pregnancy', kind: 'contraindication',
    claim: 'Fluoroquinolones (e.g. moxifloxacin) are generally contraindicated / avoided in pregnancy.' },
  { id: 'ci_tmpsmx_pregnancy', kind: 'contraindication',
    claim: 'Trimethoprim-sulfamethoxazole (TMP-SMX) is contraindicated / avoided in pregnancy (folate antagonism / neural tube risk in the first trimester; kernicterus risk near term).' },
  { id: 'ci_vancomycin_nephrotoxicity', kind: 'interaction',
    claim: 'Vancomycin is nephrotoxic, and the risk of nephrotoxicity increases when it is given with other nephrotoxic agents.' },
  { id: 'ci_aminoglycoside_vancomycin', kind: 'interaction',
    claim: 'Concurrent use of an aminoglycoside with vancomycin produces ADDITIVE / increased nephrotoxicity.' },
  { id: 'ci_fluoroquinolone_qt', kind: 'contraindication',
    claim: 'Fluoroquinolones (e.g. moxifloxacin) prolong the QT interval and should be avoided in patients with QT prolongation or who take other QT-prolonging drugs.' },
  { id: 'ci_vancomycin_renal_dose', kind: 'dose_adjustment',
    claim: 'Vancomycin requires dose adjustment and serum-level monitoring in renal impairment (it is renally cleared).' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'value_found', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    value_found: { type: 'string', description: 'the rate / strength of the contraindication or interaction as stated' },
    direction_correct: { type: 'boolean', description: 'does the source support this contraindication/interaction in this direction?' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
    discards: { type: 'array', items: { type: 'string' } },
    note: { type: 'string', description: 'ENTAILED (quote forces it) vs LEAP (inferred); explain' },
  },
}
const VERIFY_SCHEMA = {
  type: 'object',
  required: ['id', 'byte_stable', 'reextracted_value', 'refute_attempt', 'final_verdict'],
  properties: {
    id: { type: 'string' },
    byte_stable: { type: 'boolean' },
    reextracted_value: { type: 'string' },
    refute_attempt: { type: 'string' },
    final_verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
  },
}

const records = await pipeline(
  CLAIMS,
  (c) => agent(
    `Ground this antibiotic CONTRAINDICATION / INTERACTION rule against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary source — an FDA drug label (DailyMed), an IDSA guideline, ACOG (for pregnancy), or a peer-reviewed study — NOT a secondary blog. ` +
    `CLAIM [${c.id}]: ${c.claim} ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote stating the contraindication/interaction (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), value_found (the rate/strength), direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note on whether the quote ENTAILS the rule or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${c.id}`, phase: 'Ground' }
  ).then((g) => g ? { claim: c, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, reextracted_value: '', refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and supports the contraindication/interaction rule. ` +
      `CLAIM [${g.id}]: ${r.claim.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\nclaimed value: ${g.value_found}\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Re-extract the value yourself. Then make the STRONGEST refutation you can. Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return records.filter(Boolean).map((r) => ({
  id: r.claim.id, kind: r.claim.kind, claim: r.claim.claim,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))
