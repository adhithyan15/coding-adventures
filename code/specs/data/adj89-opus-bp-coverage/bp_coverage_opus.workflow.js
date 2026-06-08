export const meta = {
  name: 'adj89-bp-coverage-opus',
  description: 'Recursive byte-provenance coverage gate on the decomposed IR (Haiku only). Enumerate given inputs -> decompose -> audit unaccounted inputs -> force use-or-justified-discard -> adversarially test each discard -> if a discard fails, require the input and re-decompose -> repeat until every given input is represented or validly discarded -> solve. NO hand-tipping: the solver is never told the answer or which input matters; the gate only enforces coverage. Tests whether byte-provenance enforcement ALONE gets Haiku to the right answer on the Al(OH)3 solubility problem (it had silently dropped the given K_f).',
  phases: [{ title: 'Coverage' }, { title: 'Solve' }],
}

const M = 'opus' // the model under test — used for EVERY reasoning step
const Q = 'It is known that the K_sp of Al(OH)_3 is 5.3 * 10^(-27) and complex formation constant K_f of Al(OH)_4^(-) is 1.1 * 10^31.\n\nDetermine the solubility of Al(OH)3 in pure water, giving your answer in mol L^-1.'
const GOLD = '1.776 * 10^-3' // for the BLIND grader only; never shown to the solver

const INPUTS_SCHEMA = { type: 'object', required: ['inputs'], properties: { inputs: { type: 'array', items: { type: 'string' }, description: 'each explicitly given quantity/constant/datum, verbatim (name + value)' } } }
const IR_SCHEMA = { type: 'object', required: ['facts'], properties: { facts: { type: 'array', items: { type: 'object', required: ['id', 'statement'], properties: { id: { type: 'string' }, statement: { type: 'string', description: 'an atomic fact/equation needed to solve the problem' } } } } } }
const AUDIT_SCHEMA = { type: 'object', required: ['unaccounted'], properties: { unaccounted: { type: 'array', items: { type: 'string' }, description: 'given inputs NOT referenced/used by any fact in the decomposition (verbatim from the given-inputs list)' } } }
const RESOLVE_SCHEMA = { type: 'object', required: ['decision'], properties: { decision: { type: 'string', enum: ['use', 'discard'] }, justification: { type: 'string', description: 'if discard: why this input cannot affect the answer' } } }
const CHECK_SCHEMA = { type: 'object', required: ['verdict'], properties: { verdict: { type: 'string', enum: ['VALID', 'INVALID'], description: 'VALID = the discard is sound; INVALID = the input could affect the answer and must be used' }, reason: { type: 'string' } } }
const SOLVE_SCHEMA = { type: 'object', required: ['answer'], properties: { answer: { type: 'string' }, work: { type: 'string' } } }
const GRADE_SCHEMA = { type: 'object', required: ['grade'], properties: { grade: { type: 'string', enum: ['correct', 'incorrect', 'partial'] }, note: { type: 'string' } } }

const ag = (prompt, schema, phase, label) => agent(prompt, { model: M, agentType: 'general-purpose', schema, phase, label })

// BASELINE — plain one-shot solve (no gate), same model. This is "Opus vs Opus+framework".
const plain = await ag(`Solve this problem and give the final numeric answer with units. Show your work.\nPROBLEM: ${Q}`, SOLVE_SCHEMA, 'Solve', 'plain-solve')

// Step 1 — enumerate the given inputs (the "bytes" that must be accounted for).
const inputs = await ag(`List every explicitly given quantity, constant, or numeric datum in this problem, verbatim (name and value). Do not solve anything yet.\nPROBLEM: ${Q}`, INPUTS_SCHEMA, 'Coverage', 'enumerate-inputs')

// Step 2 — initial decomposition.
let ir = await ag(`Decompose this problem into the minimal set of atomic facts/equations needed to solve it. Do not compute the final answer yet.\nPROBLEM: ${Q}`, IR_SCHEMA, 'Coverage', 'decompose-0')

const coverage_log = []
let iter = 0
while (iter < 4) {
  iter++
  // Audit: which given inputs are NOT LOAD-BEARING (their value does not enter the answer-producing computation)?
  const audit = await ag(`Byte-provenance USE audit (load-bearing, not mere mention). GIVEN INPUTS:\n${JSON.stringify(inputs.inputs)}\nCURRENT DECOMPOSITION (facts):\n${JSON.stringify((ir.facts || []).map((f) => f.statement))}\nStep 1: identify the fact(s) that actually COMPUTE the final answer (the calculation chain). Step 2: for EACH given input, determine whether its VALUE actually ENTERS that answer-producing computation. An input that is merely named, restated, or explicitly set aside ("not needed") but whose value does NOT appear in the calculation that yields the answer is NOT load-bearing. Report EVERY given input that is not load-bearing in the answer-producing computation — regardless of any statement claiming it is unnecessary.`, AUDIT_SCHEMA, 'Coverage', `audit-${iter}`)
  const unaccounted = audit.unaccounted || []
  coverage_log.push({ iter, facts: (ir.facts || []).map((f) => f.statement), unaccounted })
  if (unaccounted.length === 0) break

  // For each unaccounted input: force use-or-justified-discard; adversarially test discards.
  const required = []
  for (const u of unaccounted) {
    const r = await ag(`Your decomposition's final computation does NOT use the given input: "${u}" (its value does not enter the calculation that yields the answer).\nRule: every given input must be either LOAD-BEARING (its value actually enters the answer calculation), or DISCARDED with a justification for why it cannot affect the answer. For "${u}", decide "use" or "discard". If "discard", give the justification.\nPROBLEM: ${Q}`, RESOLVE_SCHEMA, 'Coverage', `resolve-${iter}`)
    if (r.decision === 'use') { required.push(u); continue }
    const chk = await ag(`You are a SKEPTICAL examiner. A solver did NOT use the given input "${u}" in computing its answer, justifying:\n"${r.justification}"\nYour job is to find ANY way "${u}" could affect the answer. A problem almost never supplies a constant that genuinely does not matter, so DEFAULT TO INVALID unless you are certain "${u}" cannot affect the answer under any correct treatment. Verdict: VALID (the input genuinely cannot affect the answer) or INVALID (it could/does affect the answer and must be used).\nPROBLEM: ${Q}`, CHECK_SCHEMA, 'Coverage', `discard-check-${iter}`)
    coverage_log.push({ iter, input: u, decision: 'discard', justification: r.justification, discard_verdict: chk.verdict, reason: chk.reason })
    if (chk.verdict === 'INVALID') required.push(u)
  }
  if (required.length === 0) break // all discards accepted as valid — gate is satisfied

  // Re-decompose, making the non-discardable inputs LOAD-BEARING (value must enter the answer calculation, not just be named).
  ir = await ag(`Revise your decomposition. These given inputs CANNOT be validly discarded, so they MUST be LOAD-BEARING — their values must actually ENTER the calculation that produces the final answer, not merely be mentioned or set aside:\n${JSON.stringify(required)}\nRebuild the facts/equations so the final answer-producing computation genuinely uses these inputs' values. If using them requires a different chemical/physical model than your previous one, build that model. Do not compute the final number yet.\nPROBLEM: ${Q}\nPREVIOUS FACTS (which wrongly sidelined the above): ${JSON.stringify((ir.facts || []).map((f) => f.statement))}`, IR_SCHEMA, 'Coverage', `redecompose-${iter}`)
}

// Step 3 — solve over the (coverage-complete) IR.
const solution = await ag(`Using this decomposition, in which every given input is now accounted for, solve the problem and give the final numeric answer with units. Show your work.\nDECOMPOSITION: ${JSON.stringify((ir.facts || []).map((f) => f.statement))}\nPROBLEM: ${Q}`, SOLVE_SCHEMA, 'Solve', 'solve')

// Blind grade (Opus; never seen by the Haiku solver).
const gradeOf = (ans, label) => agent(`Grade the answer for correctness against the reference. correct / partial / incorrect.\nPROBLEM: ${Q}\nREFERENCE ANSWER: ${GOLD}\nGIVEN ANSWER: ${ans}`, { model: 'opus', agentType: 'general-purpose', schema: GRADE_SCHEMA, phase: 'Solve', label })
const [plain_grade, grade] = await Promise.all([gradeOf(plain.answer, 'grade-plain'), gradeOf(solution.answer, 'grade-gated')])

return { plain: { ...plain, grade: plain_grade }, gated: { inputs: inputs.inputs, iterations: iter, coverage_log, final_facts: (ir.facts || []).map((f) => f.statement), solution, grade } }
