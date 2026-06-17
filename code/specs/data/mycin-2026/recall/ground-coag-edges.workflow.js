// ground-coag-edges.workflow.js — ground the coagulation / bleeding-disorder edges (REL-13).
//
// The fifth recall domain (hematology). One agent grounds each disorder →
// {deficient factor, inheritance pattern, prolonged screening test} edge against a
// PRIMARY source (NIH/NCBI Bookshelf / StatPearls, a peer-reviewed reference, an
// authoritative hematology text) with a verbatim byte-quote; an independent agent
// re-fetches the cited URL and tries to refute (byte-stability). Same ground→verify
// pattern as the other domain workflows. Output → recall/coag-edge-grounding.json,
// consumed by recall/coag_edge_ground.py.

export const meta = {
  name: 'ground-coag-edges',
  description: 'Spider-ground coagulation edges (disorder→deficient factor / inheritance / prolonged screening test) against primary hematology sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per edge: WebSearch/WebFetch a primary source, verbatim byte-quote + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

// id = "<relation>__<subject>" — must match coag_edge_ground.py's edge ids.
const EDGES = [
  { id: 'factor_deficiency__hemophilia_a', rel: 'factor_deficiency', subj: 'hemophilia_a', obj: 'factor_viii',
    claim: 'Hemophilia A is caused by a deficiency of coagulation factor VIII.' },
  { id: 'coag_inheritance__hemophilia_a', rel: 'coag_inheritance', subj: 'hemophilia_a', obj: 'x_linked_recessive',
    claim: 'Hemophilia A is inherited in an X-linked recessive pattern.' },
  { id: 'prolonged_test__hemophilia_a', rel: 'prolonged_test', subj: 'hemophilia_a', obj: 'aptt',
    claim: 'Factor VIII deficiency prolongs the aPTT with a normal PT.' },
  { id: 'factor_deficiency__hemophilia_b', rel: 'factor_deficiency', subj: 'hemophilia_b', obj: 'factor_ix',
    claim: 'Hemophilia B (Christmas disease) is caused by a deficiency of coagulation factor IX.' },
  { id: 'coag_inheritance__hemophilia_b', rel: 'coag_inheritance', subj: 'hemophilia_b', obj: 'x_linked_recessive',
    claim: 'Hemophilia B is inherited in an X-linked recessive pattern.' },
  { id: 'prolonged_test__hemophilia_b', rel: 'prolonged_test', subj: 'hemophilia_b', obj: 'aptt',
    claim: 'Factor IX deficiency prolongs the aPTT with a normal PT.' },
  { id: 'factor_deficiency__von_willebrand_disease', rel: 'factor_deficiency', subj: 'von_willebrand_disease', obj: 'von_willebrand_factor',
    claim: 'Von Willebrand disease results from a deficiency of von Willebrand factor.' },
  { id: 'coag_inheritance__von_willebrand_disease', rel: 'coag_inheritance', subj: 'von_willebrand_disease', obj: 'autosomal_dominant',
    claim: 'Von Willebrand disease (type 1) is inherited in an autosomal dominant pattern.' },
  { id: 'prolonged_test__von_willebrand_disease', rel: 'prolonged_test', subj: 'von_willebrand_disease', obj: 'bleeding_time',
    claim: 'Von Willebrand disease classically prolongs the bleeding time (defective platelet adhesion).' },
  { id: 'factor_deficiency__factor_vii_deficiency', rel: 'factor_deficiency', subj: 'factor_vii_deficiency', obj: 'factor_vii',
    claim: 'Factor VII deficiency is a deficiency of coagulation factor VII.' },
  { id: 'coag_inheritance__factor_vii_deficiency', rel: 'coag_inheritance', subj: 'factor_vii_deficiency', obj: 'autosomal_recessive',
    claim: 'Factor VII deficiency is inherited in an autosomal recessive pattern.' },
  { id: 'prolonged_test__factor_vii_deficiency', rel: 'prolonged_test', subj: 'factor_vii_deficiency', obj: 'pt',
    claim: 'Factor VII deficiency prolongs the prothrombin time (PT) in isolation, with a normal aPTT.' },
  { id: 'factor_deficiency__hemophilia_c', rel: 'factor_deficiency', subj: 'hemophilia_c', obj: 'factor_xi',
    claim: 'Hemophilia C is caused by a deficiency of coagulation factor XI.' },
  { id: 'coag_inheritance__hemophilia_c', rel: 'coag_inheritance', subj: 'hemophilia_c', obj: 'autosomal_recessive',
    claim: 'Hemophilia C (factor XI deficiency) is inherited in an autosomal recessive pattern.' },
  { id: 'prolonged_test__hemophilia_c', rel: 'prolonged_test', subj: 'hemophilia_c', obj: 'aptt',
    claim: 'Factor XI deficiency prolongs the aPTT.' },
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
log(`grounding ${todo.length} coagulation edge(s); skipping ${skip.size} already grounded`)

const records = await pipeline(
  todo,
  (e) => agent(
    `Ground this coagulation FACT against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary/authoritative source — NIH/NCBI Bookshelf (e.g. StatPearls), a peer-reviewed reference, or an authoritative hematology text — NOT a secondary blog or a question bank. ` +
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
  kind: 'coag-edge-grounding',
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
