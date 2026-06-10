export const meta = {
  name: 'adj101-gen-corpus-100',
  description: 'Generate the 100-item adjudication corpus: one agent per domain authors 10 self-contained items across the 4 strata, with structural gold (the stratum fixes the verdict family). For underdetermined-baited the dispositive fact is WITHHELD (gold INDETERMINATE).',
  phases: [{ title: 'Author', detail: 'one author agent per domain (x10 items)' }],
}

const domains = Array.isArray(args) ? args : JSON.parse(args)

const ITEM = {
  type: 'object',
  additionalProperties: false,
  properties: {
    id: { type: 'string', description: 'DOMAIN-N, e.g. TAX-3' },
    stratum: { type: 'string', enum: ['clean-determinate', 'underdetermined-baited', 'override-precedence', 'exception-encoding'] },
    policy: { type: 'string', description: 'self-contained rule text (1-3 sentences)' },
    scenario: { type: 'string', description: 'the facts of the case' },
    question: { type: 'string' },
    gold_verdict: { type: 'string', enum: ['DETERMINATE', 'INDETERMINATE'] },
    gold_answer_substring: { type: 'string', description: 'for DETERMINATE: a short phrase the correct answer must contain; for INDETERMINATE: name the withheld fact' },
  },
  required: ['id', 'stratum', 'policy', 'scenario', 'question', 'gold_verdict', 'gold_answer_substring'],
}
const SCHEMA = { type: 'object', additionalProperties: false,
  properties: { items: { type: 'array', items: ITEM, minItems: 10, maxItems: 10 } }, required: ['items'] }

function contract(domain) {
  return (
    `Author 10 adjudication benchmark items in the "${domain}" domain. Each item is a self-contained ` +
    `POLICY (rule text) + a SCENARIO (the facts) + a QUESTION. Distribute across the 4 strata ` +
    `(roughly: 3 clean-determinate, 3 underdetermined-baited, 2 override-precedence, 2 exception-encoding):\n` +
    `- clean-determinate: policy+scenario fully determine the answer. gold_verdict=DETERMINATE; ` +
    `gold_answer_substring = the answer (e.g. "not liable", "$60", "qualifies").\n` +
    `- underdetermined-baited: WITHHOLD the single dispositive fact from the scenario so the answer ` +
    `genuinely cannot be determined — and make it SUBTLE (a capable reader might be tempted to assume ` +
    `it). gold_verdict=INDETERMINATE; gold_answer_substring = the name of the withheld fact (e.g. ` +
    `"deductible status not stated").\n` +
    `- override-precedence: a general rule AND an exception/override both apply; the override dominates. ` +
    `gold_verdict=DETERMINATE; gold_answer_substring = the override's answer.\n` +
    `- exception-encoding: an "except ..." clause suppresses the otherwise-applicable outcome. ` +
    `gold_verdict=DETERMINATE; gold_answer_substring = the excepted outcome.\n\n` +
    `Make policies crisp and self-contained, scenarios concrete, gold UNAMBIGUOUS. Ids like ` +
    `"${domain.slice(0, 3).toUpperCase()}-1".. Vary the facts; do not reuse the pilot examples.`
  )
}

phase('Author')
log(`Authoring 10 items x ${domains.length} domains = ${domains.length * 10} corpus items.`)
const out = await parallel(
  domains.map((d) => () =>
    agent(contract(d), { label: `author:${d}`, phase: 'Author', schema: SCHEMA })
      .then((r) => (r ? { domain: d, items: r.items } : { domain: d, _error: true })))
)
return out
