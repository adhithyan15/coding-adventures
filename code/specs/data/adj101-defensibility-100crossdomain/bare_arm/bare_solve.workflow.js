export const meta = {
  name: 'adj101-bare-arm',
  description: 'BARE arm: the model answers each item in prose directly (no framework, no program, no engine) — the head-to-head baseline. The contrast: where the framework abstains or emits an auditable program, does bare fabricate the missing fact / do math in-head?',
  phases: [{ title: 'Solve', detail: 'one bare-prose agent per item' }],
}

const parsed = Array.isArray(args) ? { items: args } : (typeof args === 'string' ? JSON.parse(args) : args)
const items = parsed.items || parsed
const MODEL = parsed.model // undefined -> inherit session model (Opus); 'haiku' for the weak-model arm

const SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    answer: { type: 'string', description: 'your final answer to the question (concise)' },
    reasoning: { type: 'string', description: 'your reasoning, in prose' },
  },
  required: ['answer', 'reasoning'],
}

function prompt(it) {
  if (it.kind === 'compute') {
    return `Answer this question. Show your reasoning.\n\n${it.source}\n\nQUESTION: ${it.question}`
  }
  return (
    `You are adjudicating. Apply the POLICY to the SCENARIO and state the determination.\n\n` +
    `POLICY: ${it.policy}\nSCENARIO: ${it.scenario}\nQUESTION: ${it.question}`
  )
}

phase('Solve')
log(`Bare arm: solving ${items.length} items in prose (no framework).`)
const out = await parallel(
  items.map((it) => () =>
    agent(prompt(it), { label: `bare:${it.id}`, phase: 'Solve', schema: SCHEMA, ...(MODEL ? { model: MODEL } : {}) })
      .then((a) => (a ? { id: it.id, kind: it.kind, ...a } : { id: it.id, _error: true })))
)
return out
