// ground-dose-windows.workflow.js — spider-ground the meningitis antibiotic DOSES.
//
// G3: retire the formulary's authored-illustrative dose debt. The dose-window
// feasibility MODEL (floor/ceiling/renal-penalty) stays a structural abstraction, but
// its ANCHOR — the guideline-recommended bacterial-meningitis (CNS) dose of each drug —
// is grounded against a primary source (IDSA/Tunkel 2004 + 2017, Sanford, drug labels),
// with a verbatim byte-quote, then an independent agent re-fetches and tries to refute.
//
// Output → grounding/dose-window-grounding.json, consumed by formulary_build.py.

export const meta = {
  name: 'ground-dose-windows',
  description: 'Spider-ground IDSA bacterial-meningitis antibiotic doses (the dose-window anchors) against primary sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per drug: WebSearch/WebFetch a guideline, verbatim byte-quote of the CNS/meningitis dose + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts the dose, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'dose_vancomycin', drug: 'vancomycin',
    claim: 'Recommended IV vancomycin dose for adult bacterial meningitis (dosed to a target serum trough), per the IDSA guideline.' },
  { id: 'dose_ceftriaxone', drug: 'ceftriaxone',
    claim: 'Recommended IV ceftriaxone dose for adult bacterial meningitis (e.g. 2 g every 12 hours), per the IDSA guideline.' },
  { id: 'dose_ampicillin', drug: 'ampicillin',
    claim: 'Recommended IV ampicillin dose for adult bacterial meningitis / Listeria (e.g. 2 g every 4 hours), per the IDSA guideline.' },
  { id: 'dose_cefepime', drug: 'cefepime',
    claim: 'Recommended IV cefepime dose for adult bacterial meningitis (e.g. 2 g every 8 hours), per the IDSA guideline.' },
  { id: 'dose_meropenem', drug: 'meropenem',
    claim: 'Recommended IV meropenem dose for adult bacterial meningitis (e.g. 2 g every 8 hours), per the IDSA guideline.' },
  { id: 'dose_moxifloxacin', drug: 'moxifloxacin',
    claim: 'Recommended IV moxifloxacin dose for adult bacterial meningitis (e.g. 400 mg daily), as an alternative/beta-lactam-sparing agent.' },
  { id: 'dose_aztreonam', drug: 'aztreonam',
    claim: 'Recommended IV aztreonam dose for adult gram-negative bacterial meningitis (e.g. 2 g every 6–8 hours), per the IDSA guideline.' },
  { id: 'dose_tmp_smx', drug: 'tmp_smx',
    claim: 'Recommended IV trimethoprim-sulfamethoxazole dose (TMP component, e.g. 5 mg/kg every 6–8 hours) for Listeria meningitis when ampicillin is contraindicated.' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'value_found', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page stating the dose — never paraphrased or fabricated' },
    value_found: { type: 'string', description: 'the dose as stated (e.g. "2 g IV every 12 hours")' },
    direction_correct: { type: 'boolean', description: 'is this the dose for BACTERIAL MENINGITIS / CNS infection (not another indication)?' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
    discards: { type: 'array', items: { type: 'string' } },
    note: { type: 'string', description: 'ENTAILED (quote states the meningitis dose) vs LEAP (inferred from another indication); explain' },
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
    `Ground this antibiotic DOSE against a PRIMARY source. Use WebSearch then WebFetch to READ a primary source — the IDSA bacterial-meningitis guideline (Tunkel 2004 Clin Infect Dis; or the 2017 healthcare-associated ventriculitis/meningitis guideline), the Sanford Guide, or the FDA drug label — NOT a secondary blog. ` +
    `CLAIM [${c.id}]: ${c.claim} ` +
    `It MUST be the dose for BACTERIAL MENINGITIS / CNS infection in adults (these are higher than ordinary doses). ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote stating the dose (never fabricate — if you cannot fetch a page with the meningitis dose, set verdict "ungrounded"), value_found (the dose), direction_correct (is it the meningitis/CNS dose?), a verdict, the sources/spans you DISCARDED (with why), and a note on whether the quote ENTAILS the meningitis dose or you made a LEAP from another indication.`,
    { schema: GROUND_SCHEMA, label: `ground:${c.id}`, phase: 'Ground' }
  ).then((g) => g ? { claim: c, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, reextracted_value: '', refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a dose grounding. WebFetch this exact URL and confirm the byte_quote really appears there and states this drug's BACTERIAL-MENINGITIS dose. ` +
      `CLAIM [${g.id}]: ${r.claim.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\nclaimed dose: ${g.value_found}\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Re-extract the dose yourself. Then make the STRONGEST refutation (is this actually the meningitis/CNS dose, or a different indication?). Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return records.filter(Boolean).map((r) => ({
  id: r.claim.id, kind: 'dose', drug: r.claim.drug, claim: r.claim.claim,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))
