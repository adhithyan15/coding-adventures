// ground-uti-id.workflow.js — spider-ground a UTI organism-identification rulebook (G4).
//
// First specialty EXPANSION beyond meningitis/bacteremia: urinary-tract infection. Same
// cold-path pattern as ground-organism-id — ground each epidemiologic prior + each
// urinalysis finding→organism association against a primary source (IDSA uncomplicated-
// cystitis guideline, StatPearls, peer-reviewed UTI etiology series), then an independent
// agent re-fetches and tries to refute (byte-stability). A new disease = point the harness
// at its sources, never hand-author a rulebook.
//
// Output → grounding/uti-id-grounding.json, consumed by the UTI write gate.

export const meta = {
  name: 'ground-uti-id',
  description: 'Spider-ground UTI organism-identification priors + urinalysis findings against primary sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per claim: WebSearch/WebFetch a primary source, verbatim byte-quote + value + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'uti_prior_ecoli', kind: 'prior', target: 'e_coli', hint: 0.78,
    claim: 'Escherichia coli is the most common cause of acute uncomplicated cystitis/UTI in women; quantify its PROPORTION of cases.' },
  { id: 'uti_prior_saprophyticus', kind: 'prior', target: 's_saprophyticus', hint: 0.10,
    claim: 'Proportion of acute uncomplicated UTI in young women caused by Staphylococcus saprophyticus.' },
  { id: 'uti_prior_klebsiella', kind: 'prior', target: 'klebsiella', hint: 0.05,
    claim: 'Proportion of uncomplicated UTI caused by Klebsiella pneumoniae.' },
  { id: 'uti_prior_proteus', kind: 'prior', target: 'proteus', hint: 0.04,
    claim: 'Proportion of uncomplicated UTI caused by Proteus mirabilis.' },
  { id: 'uti_prior_enterococcus', kind: 'prior', target: 'enterococcus', hint: 0.03,
    claim: 'Proportion of UTI caused by Enterococcus species.' },
  { id: 'uti_prior_pseudomonas', kind: 'prior', target: 'pseudomonas', hint: 0.02,
    claim: 'Proportion of COMPLICATED / catheter-associated UTI caused by Pseudomonas aeruginosa.' },
  { id: 'uti_prior_gbs', kind: 'prior', target: 'group_b_strep', hint: 0.02,
    claim: 'Proportion of UTI caused by group B Streptococcus (Streptococcus agalactiae).' },
  { id: 'uti_finding_nitrite', kind: 'finding', target: 'enterobacteriaceae',
    claim: 'A positive urine NITRITE test indicates Enterobacteriaceae (gram-negative, nitrate-reducing bacteria such as E. coli) UTI.' },
  { id: 'uti_finding_leuk_esterase', kind: 'finding', target: 'pyuria',
    claim: 'A positive urine LEUKOCYTE ESTERASE test indicates pyuria (white blood cells), supporting urinary-tract infection.' },
  { id: 'uti_finding_urease_proteus', kind: 'finding', target: 'proteus',
    claim: 'A urease-positive organism producing alkaline urine and struvite stones in UTI suggests Proteus mirabilis.' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'value_found', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    value_found: { type: 'string' },
    direction_correct: { type: 'boolean' },
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
    `Ground this clinical claim against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary source (a peer-reviewed study, the IDSA uncomplicated-cystitis/pyelonephritis guideline, or an authoritative clinical-microbiology reference — NOT a secondary blog). ` +
    `CLAIM [${c.id}]: ${c.claim}` +
    (c.hint != null ? ` (a rough prior estimate is ~${c.hint}, but DERIVE the real value from the source — do not trust the hint).` : '') +
    ` Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), value_found, direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note justifying whether the quote ENTAILS the claim or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${c.id}`, phase: 'Ground' }
  ).then((g) => g ? { claim: c, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, reextracted_value: '', refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and supports the claim. ` +
      `CLAIM [${g.id}]: ${r.claim.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\nclaimed value: ${g.value_found}\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Re-extract the value yourself. Then make the STRONGEST refutation you can. Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return records.filter(Boolean).map((r) => ({
  id: r.claim.id, kind: r.claim.kind, target: r.claim.target, claim: r.claim.claim,
  authored_hint: r.claim.hint ?? null,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))
