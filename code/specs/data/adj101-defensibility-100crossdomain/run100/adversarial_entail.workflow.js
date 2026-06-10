export const meta = {
  name: 'adj101-adversarial-entailment',
  description: 'Independent adversarial entailment read over the 240 dispositive slots: does the scenario ACTUALLY establish the extracted value, or did the extractor over-read the bytes / assert an unstated assumption? Prompted to REFUTE. Replaces the self-reported ENTAILED label.',
  phases: [{ title: 'Adjudicate-entailment', detail: 'sequential batches of 12 adversarial auditors' }],
}

const CHECKS = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj101-defensibility-100crossdomain/run100/checks'
const parsed = Array.isArray(args) ? { ids: args } : (typeof args === 'string' ? JSON.parse(args) : args)
const ids = parsed.ids || parsed
const MODEL = parsed.model // undefined -> session model (Opus); 'sonnet'/'haiku' for the N-reader panel
const BATCH = 10

const SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    check_id: { type: 'integer' },
    verdict: { type: 'string', enum: ['ENTAILED', 'LEAP'] },
    why: { type: 'string', description: '<= 40 words' },
  },
  required: ['check_id', 'verdict', 'why'],
}

function judge(cid) {
  const path = `${CHECKS}/check_${String(cid).padStart(3, '0')}.json`
  const prompt =
    `You are an ADVERSARIAL entailment auditor. Read the JSON at this path with the Read tool:\n${path}\n` +
    `It has { check_id, slot, claimed_value, scenario, cited_span }. An extractor asserts that, in this ` +
    `SCENARIO, the slot "${''}" has the claimed_value, citing cited_span.\n\n` +
    `Your job: decide whether the SCENARIO ACTUALLY ESTABLISHES that value, or whether it is an ` +
    `OVER-READING / an unstated assumption. Actively try to REFUTE that it is established — look for a ` +
    `consistent reading of the scenario in which the claimed_value is NOT established (a different ` +
    `value, or simply unknown). \n` +
    `- Return ENTAILED only if the scenario's words genuinely force the claimed_value — no reasonable ` +
    `alternative reading leaves it open.\n` +
    `- Return LEAP if the value requires an assumption, a categorization the bytes don't compel, or a ` +
    `fact the scenario never states. When in doubt, LEAP.\n\n` +
    `Echo the check_id. Return the structured verdict.`
  return agent(prompt, { label: `entail:${cid}`, phase: 'Adjudicate-entailment', schema: SCHEMA, ...(MODEL ? { model: MODEL } : {}) })
    .then((v) => (v ? { ...v, check_id: cid } : null))
}

phase('Adjudicate-entailment')
const all = []
for (let i = 0; i < ids.length; i += BATCH) {
  const batch = ids.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: entailment checks ${batch[0]}..${batch[batch.length - 1]} (${all.length} done).`)
  const res = await parallel(batch.map((cid) => () => judge(cid)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${ids.length} entailment verdicts.`)
return all
