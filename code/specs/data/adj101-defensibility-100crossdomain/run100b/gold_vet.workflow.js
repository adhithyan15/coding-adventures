export const meta = {
  name: 'adj101b-gold-vet',
  description: 'Adversarial GOLD-VET: an independent reader checks each generated gold label. For DETERMINATE gold, are ALL dispositive facts established by the scenario? For INDETERMINATE gold, does the scenario genuinely WITHHOLD the dispositive fact? Catches mislabeled items before they become an unfair yardstick. Sequential batches of 10.',
  phases: [{ title: 'Gold-vet', detail: 'sequential batches of 10 independent vetters' }],
}

const ITEMS = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/adj101-defensibility-100crossdomain/run100b/items_vet'
const indices = Array.isArray(args) ? args : JSON.parse(args)
const BATCH = 10

const SCHEMA = {
  type: 'object', additionalProperties: false,
  properties: {
    idx: { type: 'integer' },
    gold_ok: { type: 'boolean', description: 'true iff the labeled gold_verdict is correct for this item' },
    corrected_verdict: { type: 'string', enum: ['DETERMINATE', 'INDETERMINATE'], description: 'what the verdict SHOULD be' },
    why: { type: 'string', description: '<= 45 words' },
  },
  required: ['idx', 'gold_ok', 'corrected_verdict', 'why'],
}

function vet(idx) {
  const path = `${ITEMS}/item_${String(idx).padStart(3, '0')}.json`
  const prompt =
    `You are an INDEPENDENT gold-label auditor for an adjudication benchmark. Read the JSON item at ` +
    `this path with the Read tool:\n${path}\n` +
    `It has { idx, policy, scenario, question, gold_verdict, gold_answer_substring }.\n\n` +
    `Decide whether gold_verdict is CORRECT, adversarially:\n` +
    `- If gold_verdict is DETERMINATE: are ALL the facts the policy needs to decide actually ESTABLISHED ` +
    `by the scenario? If even one dispositive fact is missing / only assumable, the item is really ` +
    `INDETERMINATE and the gold is WRONG.\n` +
    `- If gold_verdict is INDETERMINATE: does the scenario genuinely WITHHOLD the dispositive fact? If the ` +
    `fact is actually established (or derivable from the bytes), the item is really DETERMINATE and the ` +
    `gold is WRONG.\n` +
    `Try to REFUTE the label. Echo the idx. Return gold_ok, the corrected_verdict, and why.`
  return agent(prompt, { label: `vet:${idx}`, phase: 'Gold-vet', schema: SCHEMA })
    .then((v) => (v ? { ...v, idx } : null))
}

phase('Gold-vet')
const all = []
for (let i = 0; i < indices.length; i += BATCH) {
  const batch = indices.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: gold-vetting items ${batch[0]}..${batch[batch.length - 1]} (${all.length} done).`)
  const res = await parallel(batch.map((idx) => () => vet(idx)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${indices.length} gold-vet verdicts.`)
return all
