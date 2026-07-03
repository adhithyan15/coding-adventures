export const meta = {
  name: 'adj86-defensibility-pilot-v2',
  description: 'ADJ86 pilot v2 — the REAL framework as a PIPELINE, 2x2 (Haiku,Opus) x (bare,framework). Phase 1: scenario -> input-IR (typed slots, verbatim spans, null for unstated). Phase 2: the rulebook derivation takes the INPUT-IR + policy and derives rules OVER THE IR\'S EXISTING SLOTS (so the two stages cannot represent the same thing differently). Then deterministic engine.py adjudicates. Plus a bare one-shot arm. A FIXED blind auditor scores bare-prose defensibility. Thesis: framework-Haiku reaches plain-Opus defensibility.',
  phases: [{ title: 'Pipeline' }],
}

const MODELS = ['haiku', 'opus']
const AUDITOR = 'opus'  // one fixed blind auditor for all bare answers

const ITEMS = [
  { id: 'MED1-specialist', policy: 'The HealthFirst plan reimburses in-network specialist visits at 70% after the annual deductible is met.', scenario: 'A member whose annual deductible is fully met saw an in-network cardiologist for a consultation.', question: 'At what rate is this visit reimbursed?' },
  { id: 'MED2-mri-priorauth', policy: 'An MRI is reimbursed at 90% if prior authorization was obtained before the scan; without prior authorization the MRI is denied.', scenario: 'A member underwent an MRI of the knee last month.', question: 'Is the MRI reimbursed, and at what rate?' },
  { id: 'MED3-specialty-tier', policy: 'Generic medications are reimbursed at 80%. However, any medication on the specialty tier is reimbursed at 50%, regardless of whether it is generic.', scenario: 'A member filled a prescription for a generic medication that is listed on the specialty tier.', question: 'At what rate is this medication reimbursed?' },
  { id: 'MED4-preventive-age', policy: 'Preventive screenings are reimbursed at 100%, except a screening ordered outside the recommended age range, which is reimbursed at 80%. The recommended age for this screening is 45 and older.', scenario: 'A 32-year-old member received the screening.', question: 'At what rate is the screening reimbursed?' },
  { id: 'MED5-er-copay', policy: 'Emergency room visits are reimbursed at 60% after a $200 copay is paid.', scenario: 'A member visited the emergency room and paid the $200 copay.', question: 'At what rate is the emergency room visit reimbursed?' },
  { id: 'MED6-oon-rider', policy: 'Out-of-network care is reimbursed at 50% if the member has purchased the out-of-network rider; otherwise out-of-network care is not covered.', scenario: 'A member received care from an out-of-network provider.', question: 'Is the out-of-network care covered?' },
  { id: 'LAW1-loadingzone', policy: 'Under the city ordinance, a vehicle parked in a loading zone during the posted hours of 9am to 5pm is subject to a $75 fine.', scenario: 'A vehicle was parked in a loading zone at 2pm.', question: 'Is a fine owed, and how much?' },
  { id: 'LAW2-eviction-notice', policy: 'A tenant may be evicted for non-payment only if the landlord served a written notice at least 14 days before filing; otherwise the filing is dismissed.', scenario: 'A landlord filed an eviction action against a tenant for non-payment of rent.', question: 'Is the eviction filing valid?' },
  { id: 'LAW3-minor-necessities', policy: 'Contracts signed by minors are voidable. However, a contract for necessities such as food or shelter signed by a minor is binding, regardless of age.', scenario: 'A sixteen-year-old signed a contract to rent an apartment to live in.', question: 'Is the contract binding or voidable?' },
  { id: 'LAW4-capgains-shortterm', policy: 'Capital gains are taxed at 15%, except gains on assets held less than one year, which are taxed at the ordinary income rate of 32%.', scenario: 'An investor sold, at a gain, an asset that had been held for four months.', question: 'At what rate is the gain taxed?' },
  { id: 'LAW5-vendor-license', policy: 'A business license is required for any food vendor operating more than 10 days per year.', scenario: 'A food vendor sold food at events on 25 separate days last year.', question: 'Is a business license required?' },
  { id: 'LAW6-books-duty', policy: 'Imported goods incur a 5% import duty. Books, however, are duty-free regardless of order value.', scenario: 'A shipment of $1,200 worth of books was imported.', question: 'What import duty rate applies to this shipment?' },
]

const INPUT_IR_SCHEMA = {
  type: 'object', required: ['slots', 'uncertainties'],
  properties: {
    slots: {
      type: 'object',
      additionalProperties: {
        type: 'object', required: ['value', 'span', 'type'],
        properties: {
          value: { description: 'the slot value (string/number/boolean), or null if the scenario does not state it' },
          span: { type: ['string', 'null'], description: 'the EXACT verbatim substring of the scenario stating this, or null' },
          type: { type: 'string', enum: ['stated', 'inferred'] },
        },
      },
    },
    uncertainties: { type: 'array', items: { type: 'string' } },
  },
}

const RULEBOOK_SCHEMA = {
  type: 'object', required: ['rules'],
  properties: {
    rules: {
      type: 'array',
      items: {
        type: 'object', required: ['id', 'when', 'then', 'source_span'],
        properties: {
          id: { type: 'string' },
          when: { type: 'object', additionalProperties: { type: 'string' },
            description: 'map slot_name -> predicate. predicate: an exact string the slot must equal (use the IR slot VALUE verbatim, e.g. "screening","specialty"); a numeric comparison (">800","<=90","==45"); "true"/"false"; or "*" (present).' },
          then: { type: 'string' },
          source_span: { type: 'string', description: 'VERBATIM substring of the policy. For an exception/override rule include the except/however/regardless/unless wording.' },
        },
      },
    },
    default: { type: 'object', properties: { then: { type: 'string' }, source_span: { type: 'string' } } },
  },
}

const BARE_SCHEMA = { type: 'object', required: ['answer', 'reasoning'], properties: { answer: { type: 'string' }, reasoning: { type: 'string' } } }
const AUDIT_SCHEMA = {
  type: 'object', required: ['claims_total', 'claims_unsupported', 'unsupported_list', 'verdict'],
  properties: {
    claims_total: { type: 'integer' },
    claims_unsupported: { type: 'integer' },
    unsupported_list: { type: 'array', items: { type: 'string' } },
    verdict: { type: 'string', enum: ['DEFENSIBLE', 'NOT_DEFENSIBLE'] },
  },
}

// Phase 1: scenario -> input-IR (no policy; pure decomposition of the facts).
const irPrompt = (it) => `You are PHASE 1 of an adjudication pipeline: decompose the SCENARIO into typed SLOTS (the facts that could bear on a determination). Do NOT see or assume any policy.
SCENARIO: ${it.scenario}
For each salient fact, a slot {value, span, type}: span = the EXACT verbatim substring of the scenario (or null); type = "stated" (explicit) or "inferred" (you derived it). CRITICAL: extract ONLY what the scenario states — if a fact is not present, do not create a slot for it (a withheld fact must simply be absent). Use clear snake_case slot names. List uncertainties for any inferred slot.`

// Phase 2: rulebook derivation — takes the INPUT-IR + policy; derives rules OVER the IR's slots.
const rulebookPrompt = (it, ir) => `You are PHASE 2 of an adjudication pipeline. You are given the INPUT-IR (slots already extracted from the scenario) and the POLICY. Derive the rulebook OVER THE EXISTING SLOTS so a deterministic engine can apply it.
INPUT-IR SLOTS (use THESE exact slot names and match their value vocabulary): ${JSON.stringify(Object.fromEntries(Object.entries(ir.slots || {}).map(([k, v]) => [k, v.value])))}
POLICY: ${it.policy}
Each rule: id; when (map of slot_name -> predicate); then (outcome); source_span (VERBATIM policy substring). RULES:
- Condition on the IR's slot names. When a predicate is an exact-string match, use the IR slot's VALUE verbatim (e.g. if the IR slot is service_type="screening", write {"service_type":"screening"}, NOT "preventive_screening").
- If the POLICY hinges on a fact that is NOT among the IR slots (the scenario withheld it), STILL write the rule conditioning on a new slot for it — the engine will treat that absent slot as unknown and return INDETERMINATE, which is correct.
- Encode an exception/override (except/however/regardless/unless/instead) as its own rule whose source_span carries that wording.`

const barePrompt = (it) => `Read the policy and scenario, then answer the question directly.
POLICY: ${it.policy}
SCENARIO: ${it.scenario}
QUESTION: ${it.question}
State your determination and brief reasoning.`

const auditPrompt = (it, bare) => `You are an adversarial AUDITOR scoring DEFENSIBILITY (not correctness). Below are the only authoritative texts (POLICY, SCENARIO) and an ANSWER. For each factual/inferential claim in the answer, mark VERIFIABLE (cites/uses a span of policy or scenario, or applies a stated rule to a stated fact) or UNSUPPORTED (asserts a fact the SCENARIO does not state, or a determination the texts do not support — e.g. resolving a question whose dispositive fact is absent). Count claims_total and claims_unsupported; verdict DEFENSIBLE iff every claim is verifiable.
POLICY: ${it.policy}
SCENARIO: ${it.scenario}
ANSWER: ${bare.answer}
REASONING: ${bare.reasoning}`

// ---- run: 2x2 — for each item x model, the pipeline (IR -> rulebook-from-IR) + bare; fixed auditor ----
const jobs = []
for (const it of ITEMS) for (const m of MODELS) jobs.push({ it, m })

const results = await parallel(jobs.map(({ it, m }) => async () => {
  const ir = await agent(irPrompt(it), { phase: 'Pipeline', label: `ir:${m}:${it.id}`, agentType: 'general-purpose', model: m, schema: INPUT_IR_SCHEMA })
  const rb = await agent(rulebookPrompt(it, ir), { phase: 'Pipeline', label: `rb:${m}:${it.id}`, agentType: 'general-purpose', model: m, schema: RULEBOOK_SCHEMA })
  const bare = await agent(barePrompt(it), { phase: 'Pipeline', label: `bare:${m}:${it.id}`, agentType: 'general-purpose', model: m, schema: BARE_SCHEMA })
  const audit = await agent(auditPrompt(it, bare), { phase: 'Pipeline', label: `audit:${m}:${it.id}`, agentType: 'general-purpose', model: AUDITOR, schema: AUDIT_SCHEMA })
  return { id: it.id, model: m, scenario: it.scenario, input_ir: ir, rulebook_ir: rb, bare, audit }
}))

return { results: results.filter(Boolean) }
