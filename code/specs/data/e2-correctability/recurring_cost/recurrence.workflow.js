export const meta = {
  name: 'e2-recurring-cost',
  description: 'E2 recurring-cost: the framework pays the policy-interpretation cost ONCE; plain prose re-pays it (unreliably) on every case. Phase 1 DeriveOnce — ONE Haiku call translates the policy into a rulebook IR (verbatim source spans; the one-time framework cost). Phase 2 Prose — one Haiku call per case answers ENTITLED/NOT_ENTITLED in free prose (the recurring arm; each call is stateless, so a correction cannot persist). Same cheap model (Haiku) on both arms.',
  phases: [{ title: 'DeriveOnce', detail: '1 Haiku call: policy -> rulebook IR' }, { title: 'Prose', detail: 'one Haiku call per case' }],
}

const DIR = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/e2-correctability/recurring_cost'
const parsed = (typeof args === 'string') ? JSON.parse(args) : args
const ids = Array.isArray(parsed) ? parsed : parsed.ids
const CORPUS = `${DIR}/${(Array.isArray(parsed) ? null : parsed.corpus) || 'corpus.json'}`
const BATCH = 10

const RULE = { type: 'object', additionalProperties: false, properties: {
  id: { type: 'string' }, when: { type: 'object', additionalProperties: { type: 'string' } },
  then: { type: 'string' }, source_span: { type: 'string' } }, required: ['id', 'when', 'then', 'source_span'] }
const RB_SCHEMA = { type: 'object', additionalProperties: false, required: ['rules'],
  properties: { rules: { type: 'array', items: RULE } } }
const PROSE_SCHEMA = { type: 'object', additionalProperties: false, required: ['verdict', 'reasoning'],
  properties: { verdict: { type: 'string', enum: ['ENTITLED', 'NOT_ENTITLED'] }, reasoning: { type: 'string', description: '<= 50 words' } } }

phase('DeriveOnce')
const derivePrompt =
  `You translate a POLICY into a rulebook IR ONCE (a deterministic engine will then decide every case). ` +
  `Read the JSON at this path with the Read tool:\n${CORPUS}\nUse its "policy" and "slots_schema".\n\n` +
  `Emit rulebook_ir rules. Each rule: a "when" (slot_name -> predicate), a "then" (the determination, ` +
  `use exactly "ENTITLED" or "NOT_ENTITLED"), and a "source_span" that is a VERBATIM substring of the ` +
  `policy. Predicates: numeric "(>=|<=|>|<|==) N", "true"/"false", exact string, or "*". The slots are ` +
  `relocation_distance_miles and destination_already_owned. Encode BOTH Section 1 (the distance rule) ` +
  `and Section 2 (the Override) as separate rules; the override's source_span must include the word ` +
  `"Override:" so precedence is explicit. Return the structured rulebook.`
const rulebook = await agent(derivePrompt, { label: 'derive-rulebook', phase: 'DeriveOnce', schema: RB_SCHEMA, model: 'haiku' })

phase('Prose')

function proseCase(cid) {
  const prompt =
    `You are answering a relocation-bonus question. Read the JSON at this path with the Read tool:\n` +
    `${CORPUS}\nFind the case with "id" == "${cid}" in its "cases" array. Using the "policy" and that ` +
    `case's "scenario" + "question", decide: is the employee ENTITLED or NOT_ENTITLED to the bonus? ` +
    `Give your verdict and brief reasoning.`
  return agent(prompt, { label: `prose:${cid}`, phase: 'Prose', schema: PROSE_SCHEMA, model: 'haiku' })
    .then((v) => (v ? { id: cid, ...v } : null))
}

const prose = []
for (let i = 0; i < ids.length; i += BATCH) {
  const batch = ids.slice(i, i + BATCH)
  log(`Prose batch ${i / BATCH + 1}: ${batch.join(', ')}`)
  const res = await parallel(batch.map((cid) => () => proseCase(cid)))
  prose.push(...res.filter(Boolean))
}

return { rulebook, prose }
