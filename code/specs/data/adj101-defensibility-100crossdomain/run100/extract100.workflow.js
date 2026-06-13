export const meta = {
  name: 'adj101-extract100',
  description: 'Framework extraction over the 100-item corpus: each agent reads its item file, translates scenario->input-IR + policy->rulebook-IR (the engine owns the verdict; the model must NOT invent a missing dispositive fact). Rate-limit-safe sequential batches.',
  phases: [{ title: 'Extract', detail: 'sequential batches of 12 extractor agents' }],
}

const ITEMS_DIR = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj101-defensibility-100crossdomain/run100/items'
const indices = Array.isArray(args) ? args : JSON.parse(args)
const BATCH = 10

const SLOT = { type: 'object', properties: {
  value: {}, span: { type: 'string' }, type: { type: 'string', enum: ['stated', 'inferred'] },
  polarity: { type: 'string', enum: ['affirmed', 'denied'] } }, required: ['value', 'type'] }
const RULE = { type: 'object', additionalProperties: false, properties: {
  id: { type: 'string' }, when: { type: 'object', additionalProperties: { type: 'string' } },
  then: { type: 'string' }, source_span: { type: 'string' } }, required: ['id', 'when', 'then', 'source_span'] }
const SCHEMA = { type: 'object', additionalProperties: false, properties: {
  idx: { type: 'integer' },
  input_ir: { type: 'object', additionalProperties: false, properties: {
    slots: { type: 'object', additionalProperties: SLOT }, uncertainties: { type: 'array', items: { type: 'string' } } },
    required: ['slots'] },
  rulebook_ir: { type: 'object', additionalProperties: false, properties: { rules: { type: 'array', items: RULE } }, required: ['rules'] },
  justifications: { type: 'array', items: { type: 'object', additionalProperties: false, properties: {
    slot: { type: 'string' }, verdict: { type: 'string', enum: ['ENTAILED', 'LEAP'] }, basis_span: { type: 'string' } },
    required: ['slot', 'verdict'] } } },
  required: ['idx', 'input_ir', 'rulebook_ir'] }

const RULES = (
  `RULES:\n` +
  `1. input-IR slots: one per fact the rules need. type "stated" carries a verbatim scenario span; ` +
  `type "inferred" carries a justification (basis_span + ENTAILED/LEAP). Use polarity "denied" for negations.\n` +
  `2. CRITICAL — do NOT invent a slot for a fact the scenario does NOT state. If a dispositive fact is ` +
  `missing, OMIT that slot: the engine returns INDETERMINATE structurally (correct). NEVER guess it.\n` +
  `3. rulebook-IR rules: each has a "when" (slot_name -> predicate), a "then" (the determination), and a ` +
  `"source_span" that is a VERBATIM substring of the POLICY. Predicates: exact string, "true"/"false", ` +
  `numeric "(>=|<=|>|<|==) N", or "*". Encode exceptions/overrides as separate rules.\n` +
  `4. A rule's "when" may reference a slot absent from input-IR (the missing fact) — that is how the ` +
  `engine knows it is underdetermined. Echo the idx from the file.`
)

function extract(idx) {
  const path = `${ITEMS_DIR}/item_${String(idx).padStart(3, '0')}.json`
  const prompt =
    `You are the EXTRACTION stage of a byte-provenance adjudication framework. You do NOT decide the ` +
    `answer — a deterministic engine fires the rules over the slots. Read the JSON item at this path ` +
    `with the Read tool:\n${path}\nIt has { idx, id, scenario, policy, question }. Translate the SCENARIO ` +
    `into input-IR slots and the POLICY into rulebook-IR rules.\n\n${RULES}\n\nReturn the structured IR.`
  return agent(prompt, { label: `extract:${idx}`, phase: 'Extract', schema: SCHEMA })
    .then((e) => (e ? { ...e, idx } : null))
}

phase('Extract')
const all = []
for (let i = 0; i < indices.length; i += BATCH) {
  const batch = indices.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: extracting items ${batch[0]}..${batch[batch.length - 1]} (${all.length} done).`)
  const res = await parallel(batch.map((idx) => () => extract(idx)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${indices.length} extractions.`)
return all
