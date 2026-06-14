export const meta = {
  name: 'adj86-defensibility-pilot-v3',
  description: 'ADJ86 pilot v3 — byte provenance on ALL legs, end to end. 2x2 (Haiku,Opus) x (bare,framework). Phase 1: scenario -> input-IR; every INFERRED slot must cite basis_span (the exact scenario bytes it is drawn from). Phase 1b: per inferred slot, an adversarial entailment gate (given those bytes, ENTAILED or world-knowledge LEAP?). Phase 2: rulebook derived OVER the IR slots, source_span verbatim in policy. The provenance-complete engine then flags only LEAP inferred conditions as auditable assumptions. Bare arm + fixed blind auditor for comparison.',
  phases: [{ title: 'Pipeline' }],
}

const MODELS = ['haiku', 'opus']
const AUDITOR = 'opus'

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
          value: { description: 'the slot value, or null if the scenario does not state it' },
          span: { type: ['string', 'null'], description: 'for a STATED slot: the EXACT verbatim substring of the scenario; null otherwise' },
          type: { type: 'string', enum: ['stated', 'inferred'] },
          basis_span: { type: ['string', 'null'], description: 'for an INFERRED slot: the EXACT verbatim scenario bytes the inference is drawn from (REQUIRED for inferred slots); null for stated slots' },
        },
      },
    },
    uncertainties: { type: 'array', items: { type: 'string' } },
  },
}

const JUSTIFY_SCHEMA = {
  type: 'object', required: ['basis_span', 'verdict'],
  properties: {
    basis_span: { type: ['string', 'null'], description: 'the EXACT verbatim scenario bytes the inference rests on, or null if none support it' },
    verdict: { type: 'string', enum: ['ENTAILED', 'LEAP'], description: 'ENTAILED iff the MEANING OF THOSE BYTES ALONE establishes the fact (restatement or deterministic computation); LEAP if it needs outside/world knowledge or an unstated assumption' },
    rationale: { type: 'string' },
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
          when: { type: 'object', additionalProperties: { type: 'string' }, description: 'map slot_name -> predicate; use the IR slot VALUE verbatim for string matches, or ">800"/"<=90"/"==45"/"true"/"false"/"*".' },
          then: { type: 'string' },
          source_span: { type: 'string', description: 'VERBATIM substring of the POLICY; include except/however/regardless/unless wording for an override.' },
        },
      },
    },
    default: { type: 'object', properties: { then: { type: 'string' }, source_span: { type: 'string' } } },
  },
}

const BARE_SCHEMA = { type: 'object', required: ['answer'], properties: { answer: { type: 'string' }, reasoning: { type: 'string' } } }
const AUDIT_SCHEMA = { type: 'object', required: ['claims_total', 'claims_unsupported', 'verdict'], properties: { claims_total: { type: 'integer' }, claims_unsupported: { type: 'integer' }, unsupported_list: { type: 'array', items: { type: 'string' } }, verdict: { type: 'string', enum: ['DEFENSIBLE', 'NOT_DEFENSIBLE'] } } }

const irPrompt = (it) => `You are PHASE 1 of an adjudication pipeline: decompose the SCENARIO into typed SLOTS (the facts that could bear on a determination). Do NOT see or assume any policy.
SCENARIO: ${it.scenario}
For each salient fact, a slot {value, span, type, basis_span}: a STATED slot sets span = the EXACT verbatim substring of the scenario (type "stated"); an INFERRED slot (type "inferred", span null) MUST set basis_span = the EXACT verbatim scenario bytes the inference is drawn from. CRITICAL: extract ONLY what the scenario states or what is genuinely derivable from it — if a fact is absent, do not create a slot for it. Use snake_case names. List uncertainties for inferred slots.`

const justifyPrompt = (it, slot, value, basis) => `You are an adversarial GROUNDING CHECKER. The SCENARIO is the ONLY ground truth. A system INFERRED:
  ${slot} = ${JSON.stringify(value)}
and pointed to these bytes as its basis: ${JSON.stringify(basis)}.
SCENARIO: ${it.scenario}
(1) Confirm or correct the basis_span: the EXACT verbatim scenario bytes the inference rests on (or null if none).
(2) Considering ONLY the meaning of those bytes — explicitly NOT outside/world knowledge — do they ENTAIL the inferred fact, or is it a LEAP beyond what the bytes literally establish? A restatement or deterministic computation (e.g. "four months" entails "less than one year") is ENTAILED. Needing a fact the bytes do not carry (e.g. that a cardiologist is a specialist, that "$" means USD, that "member" means covered) is a LEAP. Be strict.`

const rulebookPrompt = (it, ir) => `You are PHASE 2 of an adjudication pipeline. Given the INPUT-IR (slots already extracted) and the POLICY, derive the rulebook OVER THE EXISTING SLOTS so a deterministic engine can apply it.
INPUT-IR SLOTS (use THESE exact names and match their value vocabulary): ${JSON.stringify(Object.fromEntries(Object.entries(ir.slots || {}).map(([k, v]) => [k, v.value])))}
POLICY: ${it.policy}
Each rule: id; when (map slot_name -> predicate, exact-string match uses the IR slot VALUE verbatim); then; source_span (VERBATIM policy substring). If the policy hinges on a fact NOT among the IR slots, still write the rule conditioning on a new slot (the engine returns INDETERMINATE — correct). Encode exceptions/overrides (except/however/regardless/unless) as their own rule carrying that wording in source_span.`

const barePrompt = (it) => `Read the policy and scenario, then answer the question directly.
POLICY: ${it.policy}
SCENARIO: ${it.scenario}
QUESTION: ${it.question}
State your determination and brief reasoning.`

const auditPrompt = (it, bare) => `You are an adversarial AUDITOR scoring DEFENSIBILITY (not correctness). POLICY and SCENARIO are the only authoritative texts. For each claim in the ANSWER, mark VERIFIABLE (cites/uses a policy or scenario span, or applies a stated rule to a stated fact) or UNSUPPORTED (asserts a fact the scenario does not state, or a determination whose dispositive fact is absent). Count claims_total and claims_unsupported; DEFENSIBLE iff all verifiable.
POLICY: ${it.policy}
SCENARIO: ${it.scenario}
ANSWER: ${bare.answer}
REASONING: ${bare.reasoning || '(none provided)'}`

// ---- 2x2: each (item, model) runs the full provenance pipeline; inferred slots gated per-slot ----
const jobs = []
for (const it of ITEMS) for (const m of MODELS) jobs.push({ it, m })

const results = await parallel(jobs.map(({ it, m }) => async () => {
  const ir = await agent(irPrompt(it), { phase: 'Pipeline', label: `ir:${m}:${it.id}`, agentType: 'general-purpose', model: m, schema: INPUT_IR_SCHEMA })
  const inferred = Object.entries(ir.slots || {}).filter(([, v]) => v.type === 'inferred' && v.value !== null && v.value !== undefined)
  const justifications = await parallel(inferred.map(([name, sv]) => () =>
    agent(justifyPrompt(it, name, sv.value, sv.basis_span), { phase: 'Pipeline', label: `justify:${m}:${it.id}:${name}`, agentType: 'general-purpose', model: m, schema: JUSTIFY_SCHEMA }).then((v) => ({ slot: name, ...v }))))
  const rb = await agent(rulebookPrompt(it, ir), { phase: 'Pipeline', label: `rb:${m}:${it.id}`, agentType: 'general-purpose', model: m, schema: RULEBOOK_SCHEMA })
  const bare = await agent(barePrompt(it), { phase: 'Pipeline', label: `bare:${m}:${it.id}`, agentType: 'general-purpose', model: m, schema: BARE_SCHEMA })
  const audit = await agent(auditPrompt(it, bare), { phase: 'Pipeline', label: `audit:${m}:${it.id}`, agentType: 'general-purpose', model: AUDITOR, schema: AUDIT_SCHEMA })
  return { id: it.id, model: m, scenario: it.scenario, input_ir: ir, justifications: justifications.filter(Boolean), rulebook_ir: rb, bare, audit }
}))

return { results: results.filter(Boolean) }
