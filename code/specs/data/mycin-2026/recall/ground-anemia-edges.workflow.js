// ground-anemia-edges.workflow.js — ground the anemia (hematology) edges (REL-11).
//
// The third recall domain. One agent grounds each anemia → {MCV category, classic
// smear finding} edge against a PRIMARY source (a peer-reviewed reference, NIH/NCBI
// Bookshelf / StatPearls, an authoritative hematology text) with a verbatim
// byte-quote; an independent agent re-fetches and tries to refute. Same ground→verify
// pattern as the IEM/vitamin workflows. Output → recall/anemia-edge-grounding.json,
// consumed by recall/anemia_edge_ground.py.

export const meta = {
  name: 'ground-anemia-edges',
  description: 'Spider-ground anemia-classification edges (anemia→MCV category / classic smear finding) against primary hematology sources, with independent adversarial re-extraction',
  phases: [
    { title: 'Ground', detail: 'one agent per edge: WebSearch/WebFetch a primary source, verbatim byte-quote + verdict' },
    { title: 'Verify', detail: 'independent agent re-fetches the cited URL, re-extracts, attempts to refute (byte-stability)' },
  ],
}

// id = "<relation>__<subject>" — must match anemia_edge_ground.py's edge ids.
const EDGES = [
  { id: 'has_mcv__iron_deficiency_anemia', rel: 'has_mcv', subj: 'iron_deficiency_anemia', obj: 'microcytic',
    claim: 'Iron-deficiency anemia is a microcytic anemia (low MCV).' },
  { id: 'classic_finding__iron_deficiency_anemia', rel: 'classic_finding', subj: 'iron_deficiency_anemia', obj: 'koilonychia',
    claim: 'Iron-deficiency anemia can cause koilonychia (spoon nails).' },
  { id: 'has_mcv__thalassemia', rel: 'has_mcv', subj: 'thalassemia', obj: 'microcytic',
    claim: 'Thalassemia is a microcytic anemia.' },
  { id: 'classic_finding__thalassemia', rel: 'classic_finding', subj: 'thalassemia', obj: 'target_cells',
    claim: 'Thalassemia classically shows target cells on the peripheral smear.' },
  { id: 'has_mcv__sideroblastic_anemia', rel: 'has_mcv', subj: 'sideroblastic_anemia', obj: 'microcytic',
    claim: 'Sideroblastic anemia is typically microcytic.' },
  { id: 'classic_finding__sideroblastic_anemia', rel: 'classic_finding', subj: 'sideroblastic_anemia', obj: 'ringed_sideroblasts',
    claim: 'Sideroblastic anemia is defined by ringed sideroblasts in the bone marrow.' },
  { id: 'has_mcv__b12_deficiency_anemia', rel: 'has_mcv', subj: 'b12_deficiency_anemia', obj: 'macrocytic',
    claim: 'Vitamin B12 deficiency causes a macrocytic (megaloblastic) anemia.' },
  { id: 'classic_finding__b12_deficiency_anemia', rel: 'classic_finding', subj: 'b12_deficiency_anemia', obj: 'hypersegmented_neutrophils',
    claim: 'Megaloblastic anemia (B12/folate deficiency) shows hypersegmented neutrophils.' },
  { id: 'has_mcv__hereditary_spherocytosis', rel: 'has_mcv', subj: 'hereditary_spherocytosis', obj: 'normocytic',
    claim: 'Hereditary spherocytosis is a normocytic hemolytic anemia.' },
  { id: 'classic_finding__hereditary_spherocytosis', rel: 'classic_finding', subj: 'hereditary_spherocytosis', obj: 'spherocytes',
    claim: 'Hereditary spherocytosis shows spherocytes on the peripheral smear.' },
  { id: 'has_mcv__g6pd_deficiency', rel: 'has_mcv', subj: 'g6pd_deficiency', obj: 'normocytic',
    claim: 'G6PD deficiency causes an episodic normocytic hemolytic anemia.' },
  { id: 'classic_finding__g6pd_deficiency', rel: 'classic_finding', subj: 'g6pd_deficiency', obj: 'bite_cells',
    claim: 'G6PD deficiency shows bite cells (and Heinz bodies) on the peripheral smear.' },
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
// skip. The runtime may deliver args as a parsed array OR a JSON-encoded string.
function _skipList(a) {
  if (Array.isArray(a)) return a
  if (typeof a === 'string') {
    try { const p = JSON.parse(a); return Array.isArray(p) ? p : [] } catch { return [] }
  }
  return []
}
const skip = new Set(_skipList(args))
const todo = EDGES.filter((e) => !skip.has(e.id))
log(`grounding ${todo.length} anemia edge(s); skipping ${skip.size} already grounded`)

const records = await pipeline(
  todo,
  (e) => agent(
    `Ground this anemia-classification FACT against a PRIMARY source. Use WebSearch then WebFetch to actually READ a primary/authoritative source — a peer-reviewed reference, NIH/NCBI Bookshelf (e.g. StatPearls), or an authoritative hematology text — NOT a secondary blog or a question bank. ` +
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
  kind: 'anemia-edge-grounding',
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
