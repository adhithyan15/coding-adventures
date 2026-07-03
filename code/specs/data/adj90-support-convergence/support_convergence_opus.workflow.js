export const meta = {
  name: 'adj90-support-convergence-opus',
  description: 'ADJ89 inference-support gate + CONVERGENCE CONTROL. Two fixes for the oscillation that cost 1/3 on both models: (a) the support auditor categorizes each unsupported item, and a needed-but-not-given STANDARD physical constant (e.g. K_w=1e-14 at 25C) is surfaced as an explicit ASSUMPTION rather than treated as a fatal hole that makes the problem "underdetermined"; (b) the re-reason loop STOPS when the auditor stops surfacing genuinely-new issues (going in circles), instead of looping until it destabilizes. N samples, plain-Opus vs convergence-gated-Opus, on Al(OH)3. All-Opus; no hints; answer never shown to the solver.',
  phases: [{ title: 'Samples' }, { title: 'Grade' }],
}

const M = 'opus'
const N = 3
const Q = 'It is known that the K_sp of Al(OH)_3 is 5.3 * 10^(-27) and complex formation constant K_f of Al(OH)_4^(-) is 1.1 * 10^31.\n\nDetermine the solubility of Al(OH)3 in pure water, giving your answer in mol L^-1.'
const GOLD = '1.776 * 10^-3'

const IR_SCHEMA = { type: 'object', required: ['facts'], properties: { facts: { type: 'array', items: { type: 'string' } } } }
const SUPPORT_SCHEMA = { type: 'object', required: ['unsupported'], properties: { unsupported: { type: 'array', items: { type: 'object', required: ['fact_number', 'category', 'reason'], properties: { fact_number: { type: 'integer' }, category: { type: 'string', enum: ['approximation', 'contradiction', 'arithmetic', 'missing-standard-constant', 'other'], description: 'missing-standard-constant = a well-known physical constant needed but not stated in the problem (e.g. K_w, T=25C); these are ACCEPTABLE as an explicit assumption, NOT a fatal error. Other categories are genuine errors.' }, reason: { type: 'string' } } } } } }
const SOLVE_SCHEMA = { type: 'object', required: ['answer'], properties: { answer: { type: 'string' }, work: { type: 'string' } } }
const GRADE_SCHEMA = { type: 'object', required: ['grade'], properties: { grade: { type: 'string', enum: ['correct', 'partial', 'incorrect'] }, note: { type: 'string' } } }

const ags = (s, prompt, schema, label) => agent(prompt, { model: M, agentType: 'general-purpose', schema, phase: 'Samples', label: `${label}#${s}` })
const numbered = (facts) => facts.map((f, i) => `[${i + 1}] ${f}`).join('\n')
const rkey = (s) => (s || '').toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim().slice(0, 50)

const decompPrompt = `Decompose this problem into the atomic facts/inferences/equations needed to solve it (one per array entry). Do not compute the final number.\nPROBLEM: ${Q}`
const supportPrompt = (facts, assumptions) => `You are a SKEPTICAL examiner auditing a solution's inferences for SUPPORT. The PROBLEM + its givens are ground truth.\nPROBLEM: ${Q}\n${assumptions.length ? `ALREADY-DECLARED ASSUMPTIONS (do NOT re-flag these; they are accepted): ${JSON.stringify(assumptions)}\n` : ''}FACTS:\n${numbered(facts)}\nFor EACH fact decide if it is genuinely SUPPORTED. Flag every UNSUPPORTED one with a category and one-line reason. IMPORTANT category rule: if a fact needs a well-known STANDARD physical constant that the problem didn't state (e.g. K_w=1.0e-14 at 25C), categorize it 'missing-standard-constant' — that is ACCEPTABLE as an explicit assumption, NOT a reason to call the problem unsolvable. Use 'approximation'/'contradiction'/'arithmetic'/'other' only for genuine errors (an approximation the givens contradict, a wrong step, etc.). (Do not solve.)`
const reReasonPrompt = (facts, real, assumptions) => `Revise your facts. These inferences are NOT supported and must be corrected:\n${real.map((b) => `[${b.fact_number}] (${b.category}) ${b.reason}`).join('\n')}\n${assumptions.length ? `You MAY use these standard constants as EXPLICITLY-STATED ASSUMPTIONS — do NOT treat their absence from the problem as making it unsolvable: ${JSON.stringify(assumptions)}\n` : ''}If a correct treatment requires solving a coupled system instead of an approximation, set it up. Do not compute the final number.\nPROBLEM: ${Q}\nPREVIOUS FACTS:\n${numbered(facts)}`
const solvePrompt = (facts, assumptions) => `Solve the problem and give the final numeric answer with units. Show your work.${assumptions.length ? ` You may rely on these stated assumptions: ${JSON.stringify(assumptions)}.` : ''}\nFACTS:\n${numbered(facts)}\nPROBLEM: ${Q}`

const samples = await parallel(Array.from({ length: N }, (_, k) => k + 1).map((s) => async () => {
  const plain = await ags(s, `Solve this problem and give the final numeric answer with units. Show your work.\nPROBLEM: ${Q}`, SOLVE_SCHEMA, 'plain')
  let ir = await ags(s, decompPrompt, IR_SCHEMA, 'decompose')
  const assumptions = []
  const seen = new Set()
  const log = []
  let stop = ''
  for (let it = 0; it < 4; it++) {
    const a = await ags(s, supportPrompt(ir.facts || [], assumptions), SUPPORT_SCHEMA, `audit-${it}`)
    const bad = a.unsupported || []
    const missing = bad.filter((b) => b.category === 'missing-standard-constant')
    const real = bad.filter((b) => b.category !== 'missing-standard-constant')
    for (const m of missing) { const k = rkey(m.reason); if (!assumptions.some((x) => rkey(x) === k)) assumptions.push(m.reason) }
    log.push({ it, real: real.length, missing: missing.length, assumptions: [...assumptions] })
    if (!real.length) { stop = 'supported'; break }
    const fresh = real.filter((r) => !seen.has(rkey(r.reason)))
    if (!fresh.length) { stop = 'oscillation-stopped'; break } // going in circles -> stop, surface
    fresh.forEach((r) => seen.add(rkey(r.reason)))
    ir = await ags(s, reReasonPrompt(ir.facts || [], real, assumptions), IR_SCHEMA, `re-reason-${it}`)
    if (it === 3) stop = 'iter-cap'
  }
  const gated = await ags(s, solvePrompt(ir.facts || [], assumptions), SOLVE_SCHEMA, 'solve')
  return { sample: s, plain: plain.answer, gated: gated.answer, gated_facts: ir.facts, assumptions, stop, log }
}))

const gradeOf = (ans, label) => agent(`Grade for correctness vs the reference. correct/partial/incorrect.\nPROBLEM: ${Q}\nREFERENCE: ${GOLD}\nANSWER: ${ans}`, { model: 'opus', agentType: 'general-purpose', schema: GRADE_SCHEMA, phase: 'Grade', label })
const graded = await parallel(samples.filter(Boolean).map((r) => async () => ({
  sample: r.sample, stop: r.stop, assumptions: r.assumptions,
  plain: { answer: r.plain, grade: await gradeOf(r.plain, `gp#${r.sample}`) },
  gated: { answer: r.gated, grade: await gradeOf(r.gated, `gg#${r.sample}`), facts: r.gated_facts, log: r.log },
})))
const tally = (arm) => graded.filter((g) => g[arm].grade.grade === 'correct').length
return { n: N, plain_correct: tally('plain'), gated_correct: tally('gated'), results: graded }
