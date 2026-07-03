// ground-endocrine-edges.workflow.js — ground the endocrine (hormone) edges (REL-12).
//
// The fourth recall domain. One agent grounds each hormone → {secreting gland,
// deficiency syndrome} edge against a PRIMARY source (NIH/NCBI Bookshelf / StatPearls,
// a peer-reviewed reference, an authoritative endocrinology/physiology text) with a
// verbatim byte-quote; an independent agent re-fetches and tries to refute. Same
// ground→verify pattern as the other domain workflows. Output →
// recall/endocrine-edge-grounding.json, consumed by recall/endocrine_edge_ground.py.

export const meta = {
  name: 'ground-endocrine-edges',
  description: 'Spider-ground endocrine edges (hormone→secreting gland / deficiency syndrome) against primary endocrinology sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per edge: WebSearch/WebFetch a primary source, verbatim byte-quote + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

// id = "<relation>__<subject>" — must match endocrine_edge_ground.py's edge ids.
const EDGES = [
  { id: 'secreted_by__insulin', rel: 'secreted_by', subj: 'insulin', obj: 'pancreatic_beta_cells',
    claim: 'Insulin is secreted by the beta cells of the pancreatic islets.' },
  { id: 'deficiency_syndrome__insulin', rel: 'deficiency_syndrome', subj: 'insulin', obj: 'diabetes_mellitus',
    claim: 'Insulin deficiency or resistance causes diabetes mellitus.' },
  { id: 'secreted_by__cortisol', rel: 'secreted_by', subj: 'cortisol', obj: 'adrenal_cortex',
    claim: 'Cortisol is secreted by the adrenal cortex (zona fasciculata).' },
  { id: 'deficiency_syndrome__cortisol', rel: 'deficiency_syndrome', subj: 'cortisol', obj: 'addison_disease',
    claim: 'Cortisol deficiency (primary adrenal insufficiency) causes Addison disease.' },
  { id: 'secreted_by__thyroxine', rel: 'secreted_by', subj: 'thyroxine', obj: 'thyroid_gland',
    claim: 'Thyroxine (T4) is secreted by the thyroid gland.' },
  { id: 'deficiency_syndrome__thyroxine', rel: 'deficiency_syndrome', subj: 'thyroxine', obj: 'hypothyroidism',
    claim: 'Thyroid hormone deficiency causes hypothyroidism.' },
  { id: 'secreted_by__adh', rel: 'secreted_by', subj: 'adh', obj: 'posterior_pituitary',
    claim: 'Antidiuretic hormone (ADH/vasopressin) is released from the posterior pituitary.' },
  { id: 'deficiency_syndrome__adh', rel: 'deficiency_syndrome', subj: 'adh', obj: 'central_diabetes_insipidus',
    claim: 'ADH deficiency causes central diabetes insipidus.' },
  { id: 'secreted_by__pth', rel: 'secreted_by', subj: 'pth', obj: 'parathyroid_gland',
    claim: 'Parathyroid hormone is secreted by the parathyroid glands.' },
  { id: 'deficiency_syndrome__pth', rel: 'deficiency_syndrome', subj: 'pth', obj: 'hypoparathyroidism',
    claim: 'PTH deficiency causes hypoparathyroidism (hypocalcemia).' },
  { id: 'secreted_by__growth_hormone', rel: 'secreted_by', subj: 'growth_hormone', obj: 'anterior_pituitary',
    claim: 'Growth hormone is secreted by the anterior pituitary somatotrophs.' },
  { id: 'deficiency_syndrome__growth_hormone', rel: 'deficiency_syndrome', subj: 'growth_hormone', obj: 'pituitary_dwarfism',
    claim: 'Childhood growth hormone deficiency causes pituitary dwarfism (short stature).' },
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

// INCREMENTAL grounding (REL-9 fix): caller passes `args` = ids already grounded to
// skip; the runtime may deliver args as a parsed array OR a JSON-encoded string.
function _skipList(a) {
  if (Array.isArray(a)) return a
  if (typeof a === 'string') {
    try { const p = JSON.parse(a); return Array.isArray(p) ? p : [] } catch { return [] }
  }
  return []
}
const skip = new Set(_skipList(args))
const todo = EDGES.filter((e) => !skip.has(e.id))
log(`grounding ${todo.length} endocrine edge(s); skipping ${skip.size} already grounded`)

const records = await pipeline(
  todo,
  (e) => agent(
    `Ground this endocrine FACT against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary/authoritative source — NIH/NCBI Bookshelf (e.g. StatPearls), a peer-reviewed reference, or an authoritative endocrinology/physiology text — NOT a secondary blog or a question bank. ` +
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
  kind: 'endocrine-edge-grounding',
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
