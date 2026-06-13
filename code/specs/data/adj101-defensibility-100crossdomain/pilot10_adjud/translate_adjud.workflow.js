export const meta = {
  name: 'adj101-adjud-translate',
  description: 'Run 10 adjudication items through the framework EXTRACTION stage: scenario -> typed input-IR slots (provenanced), policy -> rulebook-IR rules (verbatim source_span). The deterministic engine owns the verdict; the model must NOT invent a missing dispositive fact.',
  phases: [{ title: 'Translate', detail: 'one extractor agent per item' }],
}

const items = Array.isArray(args) ? args : JSON.parse(args)

const SLOT = {
  type: 'object',
  properties: {
    value: { description: 'the typed value: string, number, or boolean' },
    span: { type: 'string', description: 'verbatim scenario bytes (REQUIRED if type=stated)' },
    type: { type: 'string', enum: ['stated', 'inferred'] },
    polarity: { type: 'string', enum: ['affirmed', 'denied'] },
  },
  required: ['value', 'type'],
}
const RULE = {
  type: 'object',
  additionalProperties: false,
  properties: {
    id: { type: 'string' },
    when: { type: 'object', additionalProperties: { type: 'string' },
            description: 'slot_name -> predicate. Predicates: exact string, "true"/"false", numeric ">=30"/"<12"/"==3", or "*" (present).' },
    then: { type: 'string', description: 'the determination this rule yields' },
    source_span: { type: 'string', description: 'verbatim substring of the POLICY this rule encodes' },
  },
  required: ['id', 'when', 'then', 'source_span'],
}
const SCHEMA = {
  type: 'object',
  additionalProperties: false,
  properties: {
    input_ir: { type: 'object', additionalProperties: false,
      properties: { slots: { type: 'object', additionalProperties: SLOT },
                    uncertainties: { type: 'array', items: { type: 'string' } } },
      required: ['slots'] },
    rulebook_ir: { type: 'object', additionalProperties: false,
      properties: { rules: { type: 'array', items: RULE } }, required: ['rules'] },
    justifications: { type: 'array', items: { type: 'object', additionalProperties: false,
      properties: { slot: { type: 'string' }, verdict: { type: 'string', enum: ['ENTAILED', 'LEAP'] },
                    basis_span: { type: 'string' } }, required: ['slot', 'verdict'] } },
  },
  required: ['input_ir', 'rulebook_ir'],
}

function contract(it) {
  return (
    `You are the EXTRACTION stage of a byte-provenance adjudication framework. You do NOT decide the ` +
    `answer — a deterministic engine does, by firing the rules over the slots. Your job: translate the ` +
    `SCENARIO into typed input-IR slots and the POLICY into rulebook-IR rules.\n\n` +
    `SCENARIO: ${JSON.stringify(it.scenario)}\nPOLICY: ${JSON.stringify(it.policy)}\n` +
    `QUESTION: ${JSON.stringify(it.question)}\n\n` +
    `RULES:\n` +
    `1. input-IR slots: one per fact the rules need. type "stated" carries a verbatim scenario span; ` +
    `type "inferred" carries a justification (basis_span + ENTAILED/LEAP). Use polarity "denied" for ` +
    `negations.\n` +
    `2. CRITICAL — do NOT invent a slot for a fact the scenario does NOT state. If a dispositive fact ` +
    `is missing from the scenario, simply omit that slot: the engine will return INDETERMINATE ` +
    `structurally (abstain), which is correct. NEVER guess the missing fact.\n` +
    `3. rulebook-IR rules: each rule has a "when" (slot_name -> predicate), a "then" (the determination), ` +
    `and a "source_span" that is a VERBATIM substring of the POLICY. Predicates: exact string match, ` +
    `"true"/"false", numeric "(>=|<=|>|<|==) N", or "*" (slot merely present). Encode exceptions/overrides ` +
    `as separate rules keyed on the exception condition.\n` +
    `4. The rule's "when" may reference a slot that is absent from input-IR (the missing dispositive ` +
    `fact) — that is how the engine knows it is underdetermined.\n\n` +
    `Return the structured IR.`
  )
}

phase('Translate')
log(`Extracting input-IR + rulebook-IR for ${items.length} adjudication items.`)
const out = await parallel(
  items.map((it) => () =>
    agent(contract(it), { label: `adjud:${it.id}`, phase: 'Translate', schema: SCHEMA })
      .then((e) => (e ? { id: it.id, ...e } : { id: it.id, _error: true })))
)
return out
