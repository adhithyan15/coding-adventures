export const meta = {
  name: 'adj89-bp-inference-support-haiku',
  description: 'The CORE of byte provenance (ADJ61 justification gate): adversarially check EVERY inference for SUPPORT, not just every discard for a reason. For each fact/inference in the IR, a skeptical auditor checks — given the problem GIVENS and the other facts — whether it genuinely follows or is an unsupported approximation/assumption/contradiction (default UNSUPPORTED). Unsupported inferences are dropped/corrected and the IR is re-derived until every inference is supported; then solve. This catches the Al(OH)3 failure that coverage gates missed: the asserted approximation "[OH-]=1e-7 because Al(OH)3 is insoluble" is contradicted by the GIVEN Kf. Run over N samples of plain-Haiku vs support-gated-Haiku (Opus is high-variance on this item). All-Opus; no hints; answer never shown to the solver.',
  phases: [{ title: 'Samples' }, { title: 'Grade' }],
}

const M = 'haiku'
const N = 3
const Q = 'It is known that the K_sp of Al(OH)_3 is 5.3 * 10^(-27) and complex formation constant K_f of Al(OH)_4^(-) is 1.1 * 10^31.\n\nDetermine the solubility of Al(OH)3 in pure water, giving your answer in mol L^-1.'
const GOLD = '1.776 * 10^-3'

const IR_SCHEMA = { type: 'object', required: ['facts'], properties: { facts: { type: 'array', items: { type: 'string', description: 'one atomic fact/inference/equation' } } } }
const SUPPORT_SCHEMA = { type: 'object', required: ['unsupported'], properties: { unsupported: { type: 'array', items: { type: 'object', required: ['fact_number', 'reason'], properties: { fact_number: { type: 'integer' }, reason: { type: 'string' } } }, description: 'every fact that is NOT supported (an unjustified approximation, an assumption the givens rule out, or a step contradicting another fact/given)' } } }
const SOLVE_SCHEMA = { type: 'object', required: ['answer'], properties: { answer: { type: 'string' }, work: { type: 'string' } } }
const GRADE_SCHEMA = { type: 'object', required: ['grade'], properties: { grade: { type: 'string', enum: ['correct', 'partial', 'incorrect'] }, note: { type: 'string' } } }

const ags = (s, prompt, schema, label) => agent(prompt, { model: M, agentType: 'general-purpose', schema, phase: 'Samples', label: `${label}#${s}` })

const decompPrompt = `Decompose this problem into the atomic facts/inferences/equations needed to solve it. One per array entry. Do not compute the final number yet.\nPROBLEM: ${Q}`
const supportPrompt = (facts) => `You are a SKEPTICAL examiner auditing a proposed solution's inferences for SUPPORT (the core check). The PROBLEM and its GIVENS are the ground truth.\nPROBLEM (with givens): ${Q}\nPROPOSED FACTS/INFERENCES:\n${facts.map((f, i) => `[${i + 1}] ${f}`).join('\n')}\nFor EACH fact, decide whether it is genuinely SUPPORTED: does it follow from the givens + basic laws WITHOUT (a) quietly approximating away a quantity the problem actually specifies, (b) assuming something the givens rule out, or (c) contradicting another fact? Be strict — DEFAULT to UNSUPPORTED for any approximation whose validity depends on a given quantity. Return every UNSUPPORTED fact by number with a one-line reason. (Do not solve.)`
const reReasonPrompt = (facts, bad) => `Revise your facts/inferences. These were judged NOT supported and must be removed or CORRECTED so every remaining inference follows from the givens with no unsupported approximation:\n${bad.map((b) => `[${b.fact_number}] reason: ${b.reason}`).join('\n')}\nIf a correct treatment requires solving a coupled system instead of an approximation, set that system up as facts. Do not compute the final number.\nPROBLEM: ${Q}\nPREVIOUS FACTS:\n${facts.map((f, i) => `[${i + 1}] ${f}`).join('\n')}`
const solvePrompt = (facts) => `Using these support-checked facts/inferences, solve the problem and give the final numeric answer with units. Show your work.\nFACTS:\n${facts.map((f, i) => `[${i + 1}] ${f}`).join('\n')}\nPROBLEM: ${Q}`

const samples = await parallel(Array.from({ length: N }, (_, k) => k + 1).map((s) => async () => {
  const plain = await ags(s, `Solve this problem and give the final numeric answer with units. Show your work.\nPROBLEM: ${Q}`, SOLVE_SCHEMA, 'plain')
  let ir = await ags(s, decompPrompt, IR_SCHEMA, 'decompose')
  const audit_log = []
  for (let it = 0; it < 3; it++) {
    const a = await ags(s, supportPrompt(ir.facts || []), SUPPORT_SCHEMA, `support-audit-${it}`)
    const bad = a.unsupported || []
    audit_log.push({ it, unsupported: bad })
    if (!bad.length) break
    ir = await ags(s, reReasonPrompt(ir.facts || [], bad), IR_SCHEMA, `re-reason-${it}`)
  }
  const gated = await ags(s, solvePrompt(ir.facts || []), SOLVE_SCHEMA, 'solve')
  return { sample: s, plain: plain.answer, gated: gated.answer, gated_facts: ir.facts, audit_log }
}))

const gradeOf = (ans, label) => agent(`Grade for correctness vs the reference. correct/partial/incorrect.\nPROBLEM: ${Q}\nREFERENCE: ${GOLD}\nANSWER: ${ans}`, { model: 'opus', agentType: 'general-purpose', schema: GRADE_SCHEMA, phase: 'Grade', label })
const graded = await parallel(samples.filter(Boolean).map((r) => async () => ({
  sample: r.sample,
  plain: { answer: r.plain, grade: await gradeOf(r.plain, `g-plain#${r.sample}`) },
  gated: { answer: r.gated, grade: await gradeOf(r.gated, `g-gated#${r.sample}`), facts: r.gated_facts, audit_log: r.audit_log },
})))

const tally = (arm) => graded.filter((g) => g[arm].grade.grade === 'correct').length
return { n: N, plain_correct: tally('plain'), gated_correct: tally('gated'), results: graded }
