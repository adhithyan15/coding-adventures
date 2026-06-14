// ground-bsi-source-lrs.workflow.js — ground the bacteremia PORTAL-OF-ENTRY LRs (G5b).
//
// The source-id rulebook's strongest signal is "which portal of entry → which bloodstream
// organism" (urinary → Enterobacterales, line → coagulase-negative staph / S. aureus,
// intra-abdominal → anaerobes, etc.) plus a couple host factors. These source→organism
// associations were authored "trust consensus". G5b grounds each against a primary source
// (IDSA, peer-reviewed BSI-by-source series), then an independent agent re-fetches + tries
// to refute. Same association-grounding pattern as the meningitis host factors (G2).
// Output → grounding/bsi-source-lr-grounding.json, consumed by the BSI source-LR gate.

export const meta = {
  name: 'ground-bsi-source-lrs',
  description: 'Spider-ground bacteremia portal-of-entry → organism associations (urinary/line/intra-abdominal/skin/respiratory + neutropenia/IDU) against primary sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per source→organism association: WebSearch/WebFetch a primary source, verbatim byte-quote + direction + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

const CLAIMS = [
  { id: 'src_urinary_enteric', evidence: 'infection_source(urinary)', target: 'enteric_gnb',
    claim: 'A urinary-tract source of bacteremia predicts Enterobacterales (enteric gram-negative bacilli, esp. Escherichia coli) as the bloodstream organism.' },
  { id: 'src_line_cons', evidence: 'infection_source(intravascular_line)', target: 'coag_neg_staph',
    claim: 'An intravascular catheter / central-line source of bacteremia predicts coagulase-negative staphylococci.' },
  { id: 'src_line_saureus', evidence: 'infection_source(intravascular_line)', target: 's_aureus',
    claim: 'An intravascular catheter / central-line source of bacteremia predicts Staphylococcus aureus.' },
  { id: 'src_intraabd_enteric', evidence: 'infection_source(intraabdominal)', target: 'enteric_gnb',
    claim: 'An intra-abdominal source of bacteremia predicts enteric gram-negative bacilli (Enterobacterales).' },
  { id: 'src_intraabd_anaerobes', evidence: 'infection_source(intraabdominal)', target: 'anaerobes',
    claim: 'An intra-abdominal source of bacteremia predicts anaerobes (e.g. Bacteroides fragilis).' },
  { id: 'src_skin_saureus', evidence: 'infection_source(skin_soft_tissue)', target: 's_aureus',
    claim: 'A skin / soft-tissue source of bacteremia predicts Staphylococcus aureus.' },
  { id: 'src_skin_pyogenes', evidence: 'infection_source(skin_soft_tissue)', target: 'strep_pyogenes',
    claim: 'A skin / soft-tissue source of bacteremia predicts group A Streptococcus (Streptococcus pyogenes).' },
  { id: 'src_resp_pneumo', evidence: 'infection_source(respiratory)', target: 's_pneumoniae',
    claim: 'A respiratory / pneumonia source of bacteremia predicts Streptococcus pneumoniae.' },
  { id: 'host_neutropenia_pseudomonas', evidence: 'neutropenia(present)', target: 'pseudomonas',
    claim: 'Neutropenia is a risk factor for Pseudomonas aeruginosa bacteremia.' },
  { id: 'host_idu_saureus', evidence: 'injection_drug_use(present)', target: 's_aureus',
    claim: 'Injection drug use is a risk factor for Staphylococcus aureus bacteremia / right-sided endocarditis.' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'value_found', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    value_found: { type: 'string', description: 'any odds ratio / relative risk / proportion the source gives for this source→organism association' },
    direction_correct: { type: 'boolean', description: 'does the source support this portal of entry RAISING the probability of this organism?' },
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
    `Ground this bacteremia portal-of-entry → organism association against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary source (an IDSA guideline, a peer-reviewed bloodstream-infection-by-source series, or an authoritative clinical-microbiology reference — NOT a secondary blog). ` +
    `CLAIM [${c.id}]: ${c.claim} Confirm the source supports that this portal of entry RAISES the likelihood of this organism (direction), and capture any odds ratio / proportion it gives. ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), value_found, direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note on whether the quote ENTAILS the association or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${c.id}`, phase: 'Ground' }
  ).then((g) => g ? { claim: c, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, reextracted_value: '', refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and supports the portal-of-entry→organism association. ` +
      `CLAIM [${g.id}]: ${r.claim.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\nclaimed value: ${g.value_found}\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Re-extract any value yourself. Then make the STRONGEST refutation (is the direction actually supported?). Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return records.filter(Boolean).map((r) => ({
  id: r.claim.id, kind: 'source_lr', evidence: r.claim.evidence, target: r.claim.target, claim: r.claim.claim,
  grounded: r.grounded,
  verify: r.verify,
  spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
    ? 'grounded'
    : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
    : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
    : 'direction_only',
}))
