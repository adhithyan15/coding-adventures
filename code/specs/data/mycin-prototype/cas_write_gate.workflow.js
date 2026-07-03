export const meta = {
  name: 'mycin-cas-write-gate',
  description: 'CAS-write gate — the INFERENCE READ. For each grounded rulebook clause, N model-diverse adversaries (Opus + Sonnet + Haiku) read the verbatim byte_quote and judge: does it ENTAIL the asserted likelihood ratio (magnitude + direction), or is the LR a LEAP beyond what the quote states? Majority vote per clause. This is what earns a clause trust before it is committed to the CAS, so warm reuse is trust-free. Sequential batches of 10 clauses (rate-limit safe).',
  phases: [{ title: 'InferenceRead', detail: '3 model-diverse readers per clause, majority vote' }],
}

const CLAUSES = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/mycin-prototype/gate/clauses'
const ids = Array.isArray(args) ? args : (typeof args === 'string' ? JSON.parse(args) : args)
const BATCH = 10
const READERS = [undefined, 'sonnet', 'haiku'] // undefined = session model (Opus)

const SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['verdict', 'why'],
  properties: {
    verdict: { type: 'string', enum: ['ENTAILED', 'LEAP'] },
    why: { type: 'string', description: '<= 40 words' },
  },
}

function reader(idx, model) {
  const path = `${CLAUSES}/clause_${String(idx).padStart(3, '0')}.json`
  const prompt =
    `You are an ADVERSARIAL provenance auditor for a clinical rulebook. Read the JSON at this ` +
    `path with the Read tool:\n${path}\nIt has { key, lr, computed, byte_quote }. A rulebook clause ` +
    `asserts that the evidence in "key" carries the likelihood ratio "lr", and claims the ` +
    `verbatim "byte_quote" supports it (via the computation in "computed").\n\n` +
    `Your job: does the byte_quote's WORDS/NUMBERS actually ENTAIL that likelihood ratio — both its ` +
    `magnitude (order of magnitude) and its direction (which way it moves the diagnosis)? Try to ` +
    `REFUTE it. Return ENTAILED only if the quote states the LR or states sensitivity/specificity ` +
    `(or counts) that compute to it. Return LEAP if the magnitude is not supported by the quote, the ` +
    `direction is wrong, or it needs an assumption the quote does not state. When in doubt, LEAP. ` +
    `Echo nothing; return the structured verdict.`
  return agent(prompt, { label: `gate:${idx}:${model || 'opus'}`, phase: 'InferenceRead', schema: SCHEMA, ...(model ? { model } : {}) })
}

function clause(idx) {
  return parallel(READERS.map((m) => () => reader(idx, m))).then((vs) => {
    const votes = vs.map((v) => (v ? v.verdict : 'LEAP'))
    const leap = votes.filter((v) => v === 'LEAP').length
    return { idx, votes, majority: leap >= 2 ? 'LEAP' : 'ENTAILED' }
  })
}

phase('InferenceRead')
const all = []
for (let i = 0; i < ids.length; i += BATCH) {
  const batch = ids.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: clauses ${batch[0]}..${batch[batch.length - 1]} (${all.length} done).`)
  const res = await parallel(batch.map((idx) => () => clause(idx)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${ids.length} clause verdicts.`)
return all
