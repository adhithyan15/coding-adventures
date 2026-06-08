export const meta = {
  name: 'adj89-bp-synthesis-gate-opus',
  description: 'Byte provenance at EVERY layer. Layer 1 (input->IR): every given input must be load-bearing or justified-discarded. Layer 2 (IR->answer SYNTHESIS): synthesis must be a CHAIN of inference steps, each declaring which facts it consumes; every IR fact must be consumed by some step or explicitly discarded-with-a-reason that survives an adversarial read. No reasoning over the whole IR in one opaque pass. Tests whether per-layer provenance gets Opus to the exact Al(OH)3 answer (plain & input-gated Opus both stalled at 5.8e-3 by silently dropping the charge-balance fact at the synthesis layer). All-Opus; no hints; answer never shown to the solver.',
  phases: [{ title: 'InputLayer' }, { title: 'SynthLayer' }, { title: 'Grade' }],
}

const M = 'opus'
const Q = 'It is known that the K_sp of Al(OH)_3 is 5.3 * 10^(-27) and complex formation constant K_f of Al(OH)_4^(-) is 1.1 * 10^31.\n\nDetermine the solubility of Al(OH)3 in pure water, giving your answer in mol L^-1.'
const GOLD = '1.776 * 10^-3'

const INPUTS_SCHEMA = { type: 'object', required: ['inputs'], properties: { inputs: { type: 'array', items: { type: 'string' } } } }
const IR_SCHEMA = { type: 'object', required: ['facts'], properties: { facts: { type: 'array', items: { type: 'object', required: ['id', 'statement'], properties: { id: { type: 'string' }, statement: { type: 'string' } } } } } }
const AUDIT_SCHEMA = { type: 'object', required: ['unaccounted'], properties: { unaccounted: { type: 'array', items: { type: 'string' } } } }
const RESOLVE_SCHEMA = { type: 'object', required: ['decision'], properties: { decision: { type: 'string', enum: ['use', 'discard'] }, justification: { type: 'string' } } }
const CHECK_SCHEMA = { type: 'object', required: ['verdict'], properties: { verdict: { type: 'string', enum: ['VALID', 'INVALID'] }, reason: { type: 'string' } } }
const SYNTH_SCHEMA = { type: 'object', required: ['steps', 'answer'], properties: { steps: { type: 'array', items: { type: 'object', required: ['inference', 'facts_used'], properties: { inference: { type: 'string' }, facts_used: { type: 'array', items: { type: 'integer' }, description: 'the numbers of the facts this step actually combines/consumes' } } } }, answer: { type: 'string' } } }
const GRADE_SCHEMA = { type: 'object', required: ['grade'], properties: { grade: { type: 'string', enum: ['correct', 'partial', 'incorrect'] }, note: { type: 'string' } } }

const ag = (prompt, schema, phase, label) => agent(prompt, { model: M, agentType: 'general-purpose', schema, phase, label })

// ---- BASELINE: plain one-shot ----
const plain = await ag(`Solve this problem and give the final numeric answer with units. Show your work.\nPROBLEM: ${Q}`, SYNTH_SCHEMA, 'Grade', 'plain-solve')

// ---- LAYER 1: input -> IR coverage ----
const inputs = await ag(`List every explicitly given quantity/constant/datum in this problem, verbatim. Do not solve.\nPROBLEM: ${Q}`, INPUTS_SCHEMA, 'InputLayer', 'enumerate')
let ir = await ag(`Decompose this problem into the minimal set of atomic facts/equations needed to solve it. Do not compute the final answer.\nPROBLEM: ${Q}`, IR_SCHEMA, 'InputLayer', 'decompose')
for (let it = 0; it < 3; it++) {
  const audit = await ag(`USE audit (load-bearing, not mere mention). GIVEN INPUTS: ${JSON.stringify(inputs.inputs)}\nFACTS: ${JSON.stringify((ir.facts || []).map((f) => f.statement))}\nReport every given input whose VALUE does not enter the answer-producing computation (ignore any "not needed" claim).`, AUDIT_SCHEMA, 'InputLayer', `audit-${it}`)
  const un = audit.unaccounted || []
  if (!un.length) break
  const need = []
  for (const u of un) {
    const r = await ag(`Given input "${u}" is not load-bearing in your decomposition. Use it, or discard with a justification for why it cannot affect the answer.\nPROBLEM: ${Q}`, RESOLVE_SCHEMA, 'InputLayer', `resolve-${it}`)
    if (r.decision === 'use') { need.push(u); continue }
    const c = await ag(`SKEPTICAL examiner. A solver did not use given input "${u}", justifying: "${r.justification}". Find ANY way it could affect the answer; DEFAULT INVALID unless certain it cannot. VALID/INVALID.\nPROBLEM: ${Q}`, CHECK_SCHEMA, 'InputLayer', `check-${it}`)
    if (c.verdict === 'INVALID') need.push(u)
  }
  if (!need.length) break
  ir = await ag(`Revise your decomposition so these given inputs are LOAD-BEARING (their values enter the answer computation): ${JSON.stringify(need)}. Do not compute the final answer.\nPROBLEM: ${Q}\nPREVIOUS: ${JSON.stringify((ir.facts || []).map((f) => f.statement))}`, IR_SCHEMA, 'InputLayer', `redecompose-${it}`)
}

// ---- LAYER 2: IR -> answer SYNTHESIS coverage ----
const factTexts = (ir.facts || []).map((f) => f.statement)
const numbered = factTexts.map((s, i) => `[${i + 1}] ${s}`).join('\n')
const synthPrompt = (extra) => `Synthesize the final numeric answer to the PROBLEM from the NUMBERED FACTS, AS A CHAIN of inference steps. Each step combines specific facts into one inference; in facts_used list the fact NUMBERS that step actually consumes. Do NOT reason over all facts in one opaque pass — show each combination, and the final answer must follow from the chain.${extra || ''}\nNUMBERED FACTS:\n${numbered}\nPROBLEM: ${Q}`
let synth = await ag(synthPrompt(''), SYNTH_SCHEMA, 'SynthLayer', 'synthesize')

const synthLog = []
for (let it = 0; it < 3; it++) {
  // DETERMINISTIC coverage: which fact numbers were never consumed by any step?
  const used = new Set((synth.steps || []).flatMap((s) => s.facts_used || []))
  const unused = factTexts.map((_, i) => i + 1).filter((n) => !used.has(n))
  synthLog.push({ it, used: [...used].sort((a, b) => a - b), unused, answer: synth.answer })
  if (!unused.length) break
  const need = []
  for (const n of unused) {
    const fact = factTexts[n - 1]
    const r = await ag(`Your inference chain never consumed this fact:\n[${n}] ${fact}\nEvery fact must be either CONSUMED by an inference step that affects the answer, or DISCARDED with a justification for why it cannot affect the answer. Decide "use" or "discard"; if discard, justify.\nPROBLEM: ${Q}`, RESOLVE_SCHEMA, 'SynthLayer', `synth-resolve-${it}-${n}`)
    if (r.decision === 'use') { need.push(n); continue }
    const c = await ag(`SKEPTICAL examiner. In solving the PROBLEM, a solver's inference chain did NOT use this fact:\n[${n}] ${fact}\nIt justifies discarding it: "${r.justification}". Find ANY way using this fact would change the final answer (e.g. a constraint/equation that is being approximated away). DEFAULT INVALID unless certain it cannot affect the answer. VALID/INVALID.\nPROBLEM: ${Q}`, CHECK_SCHEMA, 'SynthLayer', `synth-check-${it}-${n}`)
    if (c.verdict === 'INVALID') need.push(n)
  }
  if (!need.length) break
  synth = await ag(synthPrompt(`\nIMPORTANT: your previous chain failed to consume these facts, which CANNOT be validly discarded — your inference chain MUST genuinely combine them in steps that change the final answer (do not merely list their numbers): ${JSON.stringify(need)}. If consuming them requires solving a coupled system instead of an approximation, do that.`), SYNTH_SCHEMA, 'SynthLayer', `resynthesize-${it}`)
}

// ---- grade plain + gated ----
const gradeOf = (ans, label) => agent(`Grade for correctness vs the reference. correct/partial/incorrect.\nPROBLEM: ${Q}\nREFERENCE: ${GOLD}\nANSWER: ${ans}`, { model: 'opus', agentType: 'general-purpose', schema: GRADE_SCHEMA, phase: 'Grade', label })
const [plain_grade, gated_grade] = await Promise.all([gradeOf(plain.answer, 'grade-plain'), gradeOf(synth.answer, 'grade-gated')])

return {
  plain: { answer: plain.answer, grade: plain_grade },
  gated: { final_facts: factTexts, synth_log: synthLog, final_steps: synth.steps, answer: synth.answer, grade: gated_grade },
}
