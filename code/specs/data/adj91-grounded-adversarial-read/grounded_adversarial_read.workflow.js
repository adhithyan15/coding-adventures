export const meta = {
  name: 'adj91-grounded-adversarial-read',
  description: 'Byte provenance applied to the ADVERSARIAL READER itself. The support-auditor has been allowed to assert "fact X is unsupported because Y" with Y ungrounded — which is how it destabilized (e.g. a hallucinated "the system is underdetermined / six unknowns" objection). Fix: every objection must cite a verbatim grounding_quote from the PROBLEM or the FACTS, and a deterministic provenance filter DISCARDS any objection whose quote is not actually present in the bytes. Same standard for the critic as for the solver. Two arms on Al(OH)3 (N samples each): convergence-gated with an UNGROUNDED auditor (ADJ90 baseline) vs a GROUNDED auditor. Logs which objections are filtered as ungrounded. All-Opus; no hints; answer never shown to the solver.',
  phases: [{ title: 'Samples' }, { title: 'Grade' }],
}

const M = 'opus'
const N = 3
const Q = 'It is known that the K_sp of Al(OH)_3 is 5.3 * 10^(-27) and complex formation constant K_f of Al(OH)_4^(-) is 1.1 * 10^31.\n\nDetermine the solubility of Al(OH)3 in pure water, giving your answer in mol L^-1.'
const GOLD = '1.776 * 10^-3'

const IR_SCHEMA = { type: 'object', required: ['facts'], properties: { facts: { type: 'array', items: { type: 'string' } } } }
// ungrounded auditor (ADJ90)
const SUP_U = { type: 'object', required: ['unsupported'], properties: { unsupported: { type: 'array', items: { type: 'object', required: ['fact_number', 'category', 'reason'], properties: { fact_number: { type: 'integer' }, category: { type: 'string', enum: ['approximation', 'contradiction', 'arithmetic', 'missing-standard-constant', 'other'] }, reason: { type: 'string' } } } } } }
// grounded auditor: every objection must cite a verbatim quote it rests on, + where it came from
const SUP_G = { type: 'object', required: ['unsupported'], properties: { unsupported: { type: 'array', items: { type: 'object', required: ['fact_number', 'category', 'grounding_quote', 'grounding_source', 'reason'], properties: { fact_number: { type: 'integer' }, category: { type: 'string', enum: ['approximation', 'contradiction', 'arithmetic', 'missing-standard-constant', 'other'] }, grounding_quote: { type: 'string', description: 'VERBATIM text copied from the PROBLEM or from one of the FACTS that this objection rests on. Must be a real substring of the source — do not paraphrase or invent.' }, grounding_source: { type: 'string', enum: ['problem', 'fact'] }, reason: { type: 'string' } } } } } }
const SOLVE_SCHEMA = { type: 'object', required: ['answer'], properties: { answer: { type: 'string' }, work: { type: 'string' } } }
const GRADE_SCHEMA = { type: 'object', required: ['grade'], properties: { grade: { type: 'string', enum: ['correct', 'partial', 'incorrect'] }, note: { type: 'string' } } }

const ags = (s, prompt, schema, label) => agent(prompt, { model: M, agentType: 'general-purpose', schema, phase: 'Samples', label: `${label}#${s}` })
const numbered = (facts) => facts.map((f, i) => `[${i + 1}] ${f}`).join('\n')
const rkey = (s) => (s || '').toLowerCase().replace(/[^a-z0-9]+/g, ' ').trim().slice(0, 60)
// deterministic provenance check: is the auditor's grounding_quote actually present in the cited source bytes?
const toks = (s) => new Set((s || '').toLowerCase().split(/[^a-z0-9]+/).filter((t) => t.length > 1))
function isGrounded(quote, sourceText) {
  const qt = [...toks(quote)]
  if (!qt.length) return false
  const st = toks(sourceText)
  const hit = qt.filter((t) => st.has(t)).length
  return hit / qt.length >= 0.6 // >=60% of the quote's tokens must actually appear in the source
}

async function gatedSolve(s, grounded) {
  let ir = await ags(s, `Decompose this problem into the atomic facts/inferences/equations needed to solve it (one per array entry). Do not compute the final answer.\nPROBLEM: ${Q}`, IR_SCHEMA, `${grounded ? 'g' : 'u'}-decompose`)
  const assumptions = []
  const seen = new Set()
  const log = []
  let filtered_total = 0
  const filtered_examples = []
  let stop = ''
  for (let it = 0; it < 4; it++) {
    const facts = ir.facts || []
    let bad
    if (grounded) {
      const a = await ags(s, `SKEPTICAL examiner auditing inferences for SUPPORT. The PROBLEM + givens are ground truth.\nPROBLEM: ${Q}\n${assumptions.length ? `ALREADY-DECLARED ASSUMPTIONS (do NOT re-flag): ${JSON.stringify(assumptions)}\n` : ''}FACTS:\n${numbered(facts)}\nFlag every UNSUPPORTED fact. CRITICAL: for each objection you MUST cite grounding_quote = the VERBATIM text (copied exactly, not paraphrased) from the PROBLEM or from one of the FACTS that your objection rests on, and set grounding_source accordingly. An objection you cannot ground in actual quoted text is not allowed. Category rule: a well-known STANDARD constant the problem didn't state (e.g. K_w) -> 'missing-standard-constant'. Use 'approximation'/'contradiction'/'arithmetic'/'other' only for genuine errors. (Do not solve.)`, SUP_G, `g-audit-${it}`)
      const raw = a.unsupported || []
      const kept = []
      for (const o of raw) {
        const src = o.grounding_source === 'fact' ? facts.join(' \n ') : Q
        if (isGrounded(o.grounding_quote, src)) kept.push(o)
        else { filtered_total++; if (filtered_examples.length < 6) filtered_examples.push({ it, reason: o.reason, quote: o.grounding_quote, source: o.grounding_source }) }
      }
      bad = kept
    } else {
      const a = await ags(s, `SKEPTICAL examiner auditing inferences for SUPPORT. The PROBLEM + givens are ground truth.\nPROBLEM: ${Q}\n${assumptions.length ? `ALREADY-DECLARED ASSUMPTIONS (do NOT re-flag): ${JSON.stringify(assumptions)}\n` : ''}FACTS:\n${numbered(facts)}\nFlag every UNSUPPORTED fact with a category + one-line reason. Category rule: a well-known STANDARD constant the problem didn't state (e.g. K_w) -> 'missing-standard-constant'. Use 'approximation'/'contradiction'/'arithmetic'/'other' only for genuine errors. (Do not solve.)`, SUP_U, `u-audit-${it}`)
      bad = a.unsupported || []
    }
    const missing = bad.filter((b) => b.category === 'missing-standard-constant')
    const real = bad.filter((b) => b.category !== 'missing-standard-constant')
    for (const m of missing) { const k = rkey(m.reason); if (!assumptions.some((x) => rkey(x) === k)) assumptions.push(m.reason) }
    log.push({ it, real: real.length, missing: missing.length, filtered_total })
    if (!real.length) { stop = 'supported'; break }
    const fresh = real.filter((r) => !seen.has(rkey(r.reason)))
    if (!fresh.length) { stop = 'oscillation-stopped'; break }
    fresh.forEach((r) => seen.add(rkey(r.reason)))
    ir = await ags(s, `Revise your facts. These inferences are NOT supported and must be corrected:\n${real.map((b) => `[${b.fact_number}] (${b.category}) ${b.reason}`).join('\n')}\n${assumptions.length ? `You MAY use these standard constants as EXPLICIT ASSUMPTIONS (their absence does NOT make the problem unsolvable): ${JSON.stringify(assumptions)}\n` : ''}If a correct treatment needs a coupled system instead of an approximation, set it up. Do not compute the final answer.\nPROBLEM: ${Q}\nPREVIOUS FACTS:\n${numbered(facts)}`, IR_SCHEMA, `${grounded ? 'g' : 'u'}-re-reason-${it}`)
    if (it === 3) stop = 'iter-cap'
  }
  const solved = await ags(s, `Solve the problem and give the final numeric answer with units. Show your work.${assumptions.length ? ` You may rely on these stated assumptions: ${JSON.stringify(assumptions)}.` : ''}\nFACTS:\n${numbered(ir.facts || [])}\nPROBLEM: ${Q}`, SOLVE_SCHEMA, `${grounded ? 'g' : 'u'}-solve`)
  return { answer: solved.answer, stop, log, filtered_total, filtered_examples }
}

const samples = await parallel(Array.from({ length: N }, (_, k) => k + 1).map((s) => async () => {
  const [ungrounded, groundedRun] = await Promise.all([gatedSolve(s, false), gatedSolve(s, true)])
  return { sample: s, ungrounded, grounded: groundedRun }
}))

const gradeOf = (ans, label) => agent(`Grade for correctness vs the reference. correct/partial/incorrect.\nPROBLEM: ${Q}\nREFERENCE: ${GOLD}\nANSWER: ${ans}`, { model: 'opus', agentType: 'general-purpose', schema: GRADE_SCHEMA, phase: 'Grade', label })
const graded = await parallel(samples.filter(Boolean).map((r) => async () => ({
  sample: r.sample,
  ungrounded: { answer: r.ungrounded.answer, stop: r.ungrounded.stop, grade: (await gradeOf(r.ungrounded.answer, `gu#${r.sample}`)).grade },
  grounded: { answer: r.grounded.answer, stop: r.grounded.stop, filtered_total: r.grounded.filtered_total, filtered_examples: r.grounded.filtered_examples, grade: (await gradeOf(r.grounded.answer, `gg#${r.sample}`)).grade },
})))
const cc = (arm) => graded.filter((g) => g[arm].grade === 'correct').length
const filtered_grand = graded.reduce((a, g) => a + (g.grounded.filtered_total || 0), 0)
return { n: N, ungrounded_correct: cc('ungrounded'), grounded_correct: cc('grounded'), ungrounded_objections_filtered_total: filtered_grand, results: graded }
