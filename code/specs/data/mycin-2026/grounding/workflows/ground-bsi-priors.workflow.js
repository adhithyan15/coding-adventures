// ground-bsi-priors.workflow.js — spider-ground the bacteremia/BSI organism PRIORS (G5).
//
// The bacteremia source-id rulebook's base priors (which bloodstream pathogen, how often)
// are still authored "trust consensus". G5 grounds them through the same cold path as the
// meningitis priors (G1) and the UTI priors (G4): ground each proportion against a primary
// source (SCOPE/EUROBACT bloodstream-infection surveillance, IDSA, peer-reviewed BSI
// epidemiology), then an independent agent re-fetches and tries to refute (byte-stability).
// Output → grounding/bsi-prior-grounding.json, consumed by the BSI write gate (G5 gate).

export const meta = {
  name: 'ground-bsi-priors',
  description: 'Spider-ground bacteremia/BSI organism priors (which bloodstream pathogen, what proportion) against primary surveillance sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per organism prior: WebSearch/WebFetch a primary BSI-surveillance source, verbatim byte-quote + proportion + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'bsi_prior_saureus', target: 's_aureus', hint: 0.22,
    claim: 'Proportion of bloodstream isolates / bacteremia episodes caused by Staphylococcus aureus.' },
  { id: 'bsi_prior_enteric_gnb', target: 'enteric_gnb', hint: 0.25,
    claim: 'Proportion of bloodstream infections caused by Enterobacterales (enteric gram-negative bacilli, esp. Escherichia coli).' },
  { id: 'bsi_prior_cons', target: 'coag_neg_staph', hint: 0.10,
    claim: 'Proportion of positive blood cultures caused by coagulase-negative staphylococci (often line-related or contaminant).' },
  { id: 'bsi_prior_enterococcus', target: 'enterococcus', hint: 0.08,
    claim: 'Proportion of bloodstream infections caused by Enterococcus species.' },
  { id: 'bsi_prior_spneumoniae', target: 's_pneumoniae', hint: 0.07,
    claim: 'Proportion of community bloodstream infections caused by Streptococcus pneumoniae.' },
  { id: 'bsi_prior_pseudomonas', target: 'pseudomonas', hint: 0.05,
    claim: 'Proportion of bloodstream infections caused by Pseudomonas aeruginosa (healthcare-associated / neutropenic).' },
  { id: 'bsi_prior_pyogenes', target: 'strep_pyogenes', hint: 0.04,
    claim: 'Proportion of bloodstream infections caused by group A Streptococcus (Streptococcus pyogenes).' },
  { id: 'bsi_prior_candida', target: 'candida', hint: 0.03,
    claim: 'Proportion of bloodstream isolates caused by Candida species (candidemia).' },
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
    `Ground this bacteremia epidemiology claim against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary source (a bloodstream-infection surveillance study/registry — e.g. SCOPE, EUROBACT, a national BSI surveillance report — an IDSA guideline, or a peer-reviewed BSI etiology series — NOT a secondary blog). ` +
    `CLAIM [${c.id}]: ${c.claim} (a rough prior estimate is ~${c.hint}, but DERIVE the real value from the source — do not trust the hint). ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), value_found, direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note on whether the quote ENTAILS the claim or you made a LEAP.`,
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
  id: r.claim.id, kind: 'prior', target: r.claim.target, claim: r.claim.claim,
  authored_hint: r.claim.hint ?? null,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))
