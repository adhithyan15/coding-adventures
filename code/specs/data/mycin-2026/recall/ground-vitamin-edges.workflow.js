// ground-vitamin-edges.workflow.js — ground the vitamin-deficiency edges (REL-10).
//
// The second recall domain (proving the harness generalises beyond inborn errors).
// One agent grounds each vitamin → {deficiency disease, classic finding} edge against
// a PRIMARY source (a peer-reviewed reference, NIH/NCBI Bookshelf, an authoritative
// clinical-nutrition text) with a verbatim byte-quote; an independent agent re-fetches
// and tries to refute. Same ground→verify pattern as ground-iem-edges.workflow.js.
// Output → recall/vitamin-edge-grounding.json, consumed by recall/vitamin_edge_ground.py.

export const meta = {
  name: 'ground-vitamin-edges',
  description: 'Spider-ground vitamin-deficiency edges (vitamin→deficiency disease / classic finding) against primary clinical-nutrition sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per edge: WebSearch/WebFetch a primary source, verbatim byte-quote + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

// id = "<relation>__<subject>" — must match vitamin_edge_ground.py's edge ids.
const EDGES = [
  { id: 'deficiency_causes__thiamine', rel: 'deficiency_causes', subj: 'thiamine', obj: 'beriberi',
    claim: 'Thiamine (vitamin B1) deficiency causes beriberi.' },
  { id: 'classic_finding__thiamine', rel: 'classic_finding', subj: 'thiamine', obj: 'wernicke_encephalopathy',
    claim: 'Thiamine deficiency causes Wernicke encephalopathy.' },
  { id: 'deficiency_causes__niacin', rel: 'deficiency_causes', subj: 'niacin', obj: 'pellagra',
    claim: 'Niacin (vitamin B3) deficiency causes pellagra.' },
  { id: 'classic_finding__niacin', rel: 'classic_finding', subj: 'niacin', obj: 'dermatitis_diarrhea_dementia',
    claim: 'Pellagra (niacin deficiency) presents with dermatitis, diarrhea, and dementia.' },
  { id: 'deficiency_causes__cobalamin', rel: 'deficiency_causes', subj: 'cobalamin', obj: 'megaloblastic_anemia',
    claim: 'Cobalamin (vitamin B12) deficiency causes megaloblastic anemia.' },
  { id: 'classic_finding__cobalamin', rel: 'classic_finding', subj: 'cobalamin', obj: 'subacute_combined_degeneration',
    claim: 'Vitamin B12 deficiency causes subacute combined degeneration of the spinal cord.' },
  { id: 'deficiency_causes__folate', rel: 'deficiency_causes', subj: 'folate', obj: 'megaloblastic_anemia',
    claim: 'Folate (vitamin B9) deficiency causes megaloblastic anemia.' },
  { id: 'classic_finding__folate', rel: 'classic_finding', subj: 'folate', obj: 'neural_tube_defects',
    claim: 'Periconceptional folate deficiency is associated with fetal neural tube defects.' },
  { id: 'deficiency_causes__vitamin_c', rel: 'deficiency_causes', subj: 'vitamin_c', obj: 'scurvy',
    claim: 'Vitamin C (ascorbic acid) deficiency causes scurvy.' },
  { id: 'classic_finding__vitamin_c', rel: 'classic_finding', subj: 'vitamin_c', obj: 'impaired_collagen_synthesis',
    claim: 'Vitamin C is required for collagen hydroxylation; its deficiency impairs collagen synthesis.' },
  { id: 'deficiency_causes__vitamin_d', rel: 'deficiency_causes', subj: 'vitamin_d', obj: 'rickets',
    claim: 'Vitamin D deficiency causes rickets in children.' },
  { id: 'classic_finding__vitamin_d', rel: 'classic_finding', subj: 'vitamin_d', obj: 'osteomalacia',
    claim: 'Vitamin D deficiency causes osteomalacia in adults.' },
  { id: 'deficiency_causes__vitamin_a', rel: 'deficiency_causes', subj: 'vitamin_a', obj: 'night_blindness',
    claim: 'Vitamin A deficiency causes night blindness (nyctalopia).' },
  { id: 'classic_finding__vitamin_a', rel: 'classic_finding', subj: 'vitamin_a', obj: 'xerophthalmia',
    claim: 'Vitamin A deficiency causes xerophthalmia.' },
  { id: 'deficiency_causes__vitamin_k', rel: 'deficiency_causes', subj: 'vitamin_k', obj: 'coagulopathy',
    claim: 'Vitamin K deficiency causes a bleeding diathesis (coagulopathy).' },
  { id: 'classic_finding__vitamin_k', rel: 'classic_finding', subj: 'vitamin_k', obj: 'prolonged_prothrombin_time',
    claim: 'Vitamin K deficiency prolongs the prothrombin time (PT/INR).' },
]

const GROUND_SCHEMA = {
  type: 'object',
  required: ['id', 'resolved_url', 'source_title', 'byte_quote', 'direction_correct', 'verdict', 'discards', 'note'],
  properties: {
    id: { type: 'string' },
    resolved_url: { type: 'string' },
    source_title: { type: 'string' },
    byte_quote: { type: 'string', description: 'VERBATIM sentence(s) copied from the fetched page — never paraphrased or fabricated' },
    direction_correct: { type: 'boolean', description: 'does the source state this edge (subject → object)?' },
    verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
    discards: { type: 'array', items: { type: 'string' } },
    note: { type: 'string', description: 'ENTAILED (quote forces it) vs LEAP (inferred); explain' },
  },
}
const VERIFY_SCHEMA = {
  type: 'object',
  required: ['id', 'byte_stable', 'refute_attempt', 'final_verdict'],
  properties: {
    id: { type: 'string' },
    byte_stable: { type: 'boolean' },
    refute_attempt: { type: 'string' },
    final_verdict: { type: 'string', enum: ['grounded', 'direction_only', 'refuted', 'ungrounded'] },
  },
}

// INCREMENTAL grounding (same as the IEM workflow, REL-9 fix): the caller passes
// `args` = ids already grounded to skip. The runtime may deliver args as a parsed
// array OR a JSON-encoded string, so normalise before building the skip-set.
function _skipList(a) {
  if (Array.isArray(a)) return a
  if (typeof a === 'string') {
    try { const p = JSON.parse(a); return Array.isArray(p) ? p : [] } catch { return [] }
  }
  return []
}
const skip = new Set(_skipList(args))
const todo = EDGES.filter((e) => !skip.has(e.id))
log(`grounding ${todo.length} vitamin edge(s); skipping ${skip.size} already grounded`)

const records = await pipeline(
  todo,
  (e) => agent(
    `Ground this vitamin-deficiency FACT against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary/authoritative source — a peer-reviewed reference, NIH/NCBI Bookshelf (e.g. StatPearls), or an authoritative clinical-nutrition / biochemistry text — NOT a secondary blog or a question bank. ` +
    `FACT [${e.id}]: ${e.claim} Confirm the source states this exact relation (${e.subj} → ${e.obj}). ` +
    `Return the resolved_url you fetched, source_title, a VERBATIM byte_quote from that page (never fabricate — if you cannot fetch a page with a supporting quote, set verdict "ungrounded"), direction_correct, a verdict, the sources/spans you DISCARDED (with why), and a note on whether the quote ENTAILS the fact or you made a LEAP.`,
    { schema: GROUND_SCHEMA, label: `ground:${e.id}`, phase: 'Ground' }
  ).then((g) => g ? { edge: e, grounded: g } : null),
  (r) => {
    if (!r || !r.grounded) return null
    const g = r.grounded
    if (g.verdict === 'ungrounded') return { ...r, verify: { id: g.id, byte_stable: false, refute_attempt: 'n/a', final_verdict: 'ungrounded' } }
    return agent(
      `Independently VERIFY a grounding. WebFetch this exact URL and confirm the byte_quote really appears there and states the edge. ` +
      `FACT [${g.id}]: ${r.edge.claim}\nURL: ${g.resolved_url}\nbyte_quote to confirm (verbatim): "${g.byte_quote}"\n` +
      `Set byte_stable=true ONLY if the quote appears verbatim on the page you fetch. Then make the STRONGEST refutation (is the relation actually stated?). Give your final_verdict.`,
      { schema: VERIFY_SCHEMA, label: `verify:${g.id}`, phase: 'Verify' }
    ).then((v) => ({ ...r, verify: v }))
  }
)

return {
  kind: 'vitamin-edge-grounding',
  records: records.filter(Boolean).map((r) => ({
    id: r.edge.id, relation: r.edge.rel, subject: r.edge.subj, object: r.edge.obj, claim: r.edge.claim,
    grounded: r.grounded,
    verify: r.verify,
    spider_status: (r.verify && r.verify.byte_stable && r.grounded.verdict === 'grounded' && r.verify.final_verdict === 'grounded')
      ? 'grounded'
      : (r.grounded.verdict === 'refuted' || (r.verify && r.verify.final_verdict === 'refuted')) ? 'refuted'
      : (r.grounded.verdict === 'ungrounded') ? 'ungrounded'
      : 'direction_only',
  })),
}
