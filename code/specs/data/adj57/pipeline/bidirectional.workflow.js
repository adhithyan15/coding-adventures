export const meta = {
  name: 'adj63-bidirectional-end-to-end',
  description: 'ADJ61+ADJ62 end-to-end on a FRESH case. INPUT: decompose (coverage) -> account for every fact extracted/inferred with the bytes + justification -> input justification gate + kickback. OUTPUT: derive the answer as evidence/conclusion claims grounded by COMBINING input bytes -> output justification gate + kickback. Picks a brand-new documented case (prefer a non-medical identification domain) to test that the bidirectional justification gate generalizes.',
  phases: [{ title: 'Ingest' }, { title: 'ExtractInput' }, { title: 'VerifyInput' }, { title: 'KickbackInput' }, { title: 'Derive' }, { title: 'VerifyOutput' }, { title: 'KickbackOutput' }],
}

const INGEST_SCHEMA = {
  type: 'object',
  required: ['source_url', 'domain', 'ground_truth', 'case_text', 'segments'],
  properties: {
    source_url: { type: 'string' },
    domain: { type: 'string', description: 'the identification domain, e.g. mineralogy, materials-failure, forensic-entomology, numismatics, ornithology, medicine' },
    ground_truth: { type: 'string', description: 'the confirmed answer + how established. Held aside from all later stages.' },
    case_text: { type: 'string', description: 'the EXACT scenario text decomposed (~500-1500 chars): observed evidence up to the decision point, NOT the named answer. segments MUST concatenate to this exactly.' },
    segments: {
      type: 'array',
      items: {
        type: 'object', required: ['text', 'kind'],
        properties: { text: { type: 'string' }, kind: { type: 'string', enum: ['fact', 'discard'] }, reason: { type: 'string' } },
      },
    },
  },
}
const FACTS_SCHEMA = {
  type: 'object', required: ['facts'],
  properties: {
    facts: {
      type: 'array',
      items: {
        type: 'object', required: ['fact', 'kind', 'grounded_by', 'justification'],
        properties: {
          fact: { type: 'string' }, kind: { type: 'string', enum: ['extracted', 'inferred'] },
          grounded_by: { type: 'array', items: { type: 'string' } }, justification: { type: 'string' },
        },
      },
    },
  },
}
const DERIVE_SCHEMA = {
  type: 'object', required: ['leading_answer', 'claims'],
  properties: {
    leading_answer: { type: 'string' },
    claims: {
      type: 'array',
      items: {
        type: 'object', required: ['claim', 'kind', 'grounded_by'],
        properties: {
          claim: { type: 'string' }, kind: { type: 'string', enum: ['evidence', 'conclusion'] },
          grounded_by: { type: 'array', items: { type: 'string' } },
        },
      },
    },
  },
}
const VERIFY_SCHEMA = {
  type: 'object', required: ['verdicts'],
  properties: {
    verdicts: {
      type: 'array',
      items: {
        type: 'object', required: ['idx', 'justified', 'note'],
        properties: { idx: { type: 'integer' }, justified: { type: 'boolean' }, note: { type: 'string' } },
      },
    },
  },
}

const ingestPrompt = `Find ONE real documented identification/diagnosis case to reason about. PREFER a domain OTHER than infectious-disease medicine — e.g. mineralogy/gemstone ID, materials/structural failure analysis, forensic entomology, numismatics (coin authentication), ornithology, meteoritics. Decompose its SCENARIO into a byte-covering IR.
1. Choose a self-contained scenario passage (~500-1500 chars): the observed evidence up to the decision point. Do NOT include the named final answer. Copy VERBATIM into case_text.
2. Partition case_text into ordered segments; concatenating segment.text must reproduce case_text EXACTLY (every char). Each is a fact or a discard (with reason).
3. Return source_url, domain, and the held-aside ground_truth (the confirmed answer + how established).`

const extractPrompt = (caseText) => `You have decomposed the case below. Now account for what you took from it.

CASE TEXT:
${caseText}

List EVERY fact you extracted or inferred. For each: the fact; its kind — "extracted" (the bytes STATE it directly) or "inferred" (you DERIVED it, a reading/interpretation); grounded_by = byte span(s) copied VERBATIM from the case text (combine several if the fact is built from several); and a justification of why those exact bytes PROVE the fact (extracted) or WARRANT it (inferred). Be honest about kind: an interpretation or label you add is INFERRED, not extracted. Do NOT claim to have extracted a fact the bytes do not state.`

const verifyInputPrompt = (caseText, facts) => `You are an adversarial extraction verifier. For EACH fact, decide whether the CITED bytes — and only those — justify it at its stated kind. Try to REFUTE each; default justified=false when the cited bytes do not carry it.

THE CASE TEXT (the only ground truth):
${caseText}

Rules:
  - extracted: the cited bytes must STATE the fact. If it adds any reading the bytes do not contain, justified=false (should be inferred).
  - inferred: the cited bytes, combined, must WARRANT it as a defensible interpretation (not a leap).

FACTS:
${facts.map((f, i) => `  [${i}] kind=${f.kind} :: "${f.fact}"\n       cites: ${JSON.stringify(f.grounded_by)}\n       justification: ${f.justification}`).join('\n')}`

const derivePrompt = (caseText, facts) => `Answer this case. You may COMBINE several input bytes into one fact, and you may state a CONCLUSION — but every claim must be JUSTIFIED by the input bytes it cites.

CASE TEXT:
${caseText}

GROUNDED FACTS established from the input (you may rely on these):
${facts.map((f) => `  - [${f.kind}] ${f.fact}`).join('\n')}

Emit leading_answer + claims. Type EACH claim:
  - kind="evidence": a statement ABOUT the input; its cited bytes must state or directly imply it. Do NOT assert a finding the case does not contain.
  - kind="conclusion": an INFERENCE (the identification/diagnosis, a mechanism, a unifying pattern). You MAY name it even if the name is not a byte — PROVIDED it rests only on byte-grounded evidence and is stated as a hedged inference (a leading hypothesis), not a byte-fact.
Every claim's grounded_by must be spans copied VERBATIM from the CASE TEXT above. The leading_answer must be a hedged conclusion the claims justify.`

const verifyOutputPrompt = (caseText, claims) => `You are an adversarial grounding verifier. For EACH claim, decide whether the CITED bytes — read together — JUSTIFY it at its stated strength. Try to REFUTE each; default justified=false when the cited bytes do not carry the claim.

THE INPUT (the only ground truth):
${caseText}

Rules:
  - evidence: the cited bytes must STATE or directly imply the claim; an asserted finding not in the bytes is invention -> false.
  - conclusion: the cited bytes, COMBINED, must make it the WARRANTED reading AND it must be hedged as an inference (not asserted as confirmed). If a different explanation fits the same bytes equally, or it over-asserts, justified=false.

CLAIMS:
${claims.map((c, i) => `  [${i}] kind=${c.kind} :: "${c.claim}"\n       cites: ${JSON.stringify(c.grounded_by)}`).join('\n')}`

const kickbackInputPrompt = (rejected) => `The INPUT justification gate REJECTED these fact extractions:
${rejected.map((r) => `  - [${r.kind}] "${r.fact}"\n      reason: ${r.reason}`).join('\n')}
Fix EACH by: (a) cite the exact byte span(s) that prove it (verbatim; combine if needed); (b) if you labelled an interpretation "extracted", re-file as kind="inferred" with a justification; (c) if the bytes do not support it, REMOVE it. Re-emit the FULL corrected facts list.`

const kickbackOutputPrompt = (prev, rejected) => `The OUTPUT justification gate REJECTED these claims:
${rejected.map((r) => `  - [${r.kind}] "${r.claim}"\n      reason: ${r.reason}`).join('\n')}
Your previous leading_answer: "${prev.leading_answer}".
Fix EACH by: (a) cite span(s) copied EXACTLY from the case text that justify it (combine if needed); (b) if it is an over-asserted conclusion, SOFTEN to a hedged inference; (c) if the input does not support it, REMOVE it (and any specificity it lent the answer). Re-emit the FULL corrected leading_answer + claims.`

// ---- gates (mirror of justify_gate.py) ----
const anchoredIn = (hay, c) => {
  const spans = (c.grounded_by || []).filter(Boolean)
  return spans.length > 0 && spans.every((s) => hay.includes(s))
}
async function verifyStage(promptFn, items, phase) {
  const v = await agent(promptFn(items), { phase, label: phase.toLowerCase(), agentType: 'general-purpose', schema: VERIFY_SCHEMA })
  return new Map((v.verdicts || []).map((x) => [x.idx, x]))
}
function gradeItems(hay, items, verdicts, textKey) {
  return items.map((it, i) => {
    const verdict = verdicts.get(i) || { justified: false, note: 'no verdict returned' }
    const isAnchored = anchoredIn(hay, it)
    const grounded = isAnchored && !!verdict.justified
    const reason = grounded ? '' : (!isAnchored ? 'byte-anchor FAIL — a cited span is not verbatim (or no citation)' : `justification FAIL — ${verdict.note}`)
    return { ...it, [textKey]: it[textKey], anchored: isAnchored, justified: !!verdict.justified, verifier_note: verdict.note, grounded, reason }
  })
}

// ---- run ----
// INPUT
const ingest = await agent(ingestPrompt, { phase: 'Ingest', label: 'ingest', agentType: 'general-purpose', schema: INGEST_SCHEMA })
const caseText = ingest.case_text
const coverage_ok = ingest.segments.map((s) => s.text).join('') === caseText
log(`domain=${ingest.domain}; coverage ${coverage_ok ? 'OK' : 'BROKEN'} (${caseText.length} bytes)`)

let facts = (await agent(extractPrompt(caseText), { phase: 'ExtractInput', label: 'extract', agentType: 'general-purpose', schema: FACTS_SCHEMA })).facts
let inGraded = gradeItems(caseText, facts, await verifyStage((f) => verifyInputPrompt(caseText, f), facts, 'VerifyInput'), 'fact')
const inputAttempts = [{ attempt: 1, n: inGraded.length, n_rejected: inGraded.filter((g) => !g.grounded).length }]
for (let i = 2; i <= 4; i++) {
  const rej = inGraded.filter((g) => !g.grounded)
  if (!rej.length) break
  log(`INPUT gate: ${rej.length} rejected -> kickback ${i}`)
  facts = (await agent(kickbackInputPrompt(rej), { phase: 'KickbackInput', label: `re-extract-${i}`, agentType: 'general-purpose', schema: FACTS_SCHEMA })).facts
  inGraded = gradeItems(caseText, facts, await verifyStage((f) => verifyInputPrompt(caseText, f), facts, 'VerifyInput'), 'fact')
  inputAttempts.push({ attempt: i, n: inGraded.length, n_rejected: inGraded.filter((g) => !g.grounded).length })
}
const groundedFacts = inGraded.filter((g) => g.grounded)

// OUTPUT
let derived = await agent(derivePrompt(caseText, groundedFacts), { phase: 'Derive', label: 'derive', agentType: 'general-purpose', schema: DERIVE_SCHEMA })
let outGraded = gradeItems(caseText, derived.claims, await verifyStage((c) => verifyOutputPrompt(caseText, c), derived.claims, 'VerifyOutput'), 'claim')
const outputAttempts = [{ attempt: 1, leading_answer: derived.leading_answer, n: outGraded.length, n_rejected: outGraded.filter((g) => !g.grounded).length }]
for (let i = 2; i <= 4; i++) {
  const rej = outGraded.filter((g) => !g.grounded)
  if (!rej.length) break
  log(`OUTPUT gate: ${rej.length} rejected -> kickback ${i}`)
  derived = await agent(kickbackOutputPrompt(derived, rej), { phase: 'KickbackOutput', label: `re-derive-${i}`, agentType: 'general-purpose', schema: DERIVE_SCHEMA })
  outGraded = gradeItems(caseText, derived.claims, await verifyStage((c) => verifyOutputPrompt(caseText, c), derived.claims, 'VerifyOutput'), 'claim')
  outputAttempts.push({ attempt: i, leading_answer: derived.leading_answer, n: outGraded.length, n_rejected: outGraded.filter((g) => !g.grounded).length })
}

return {
  domain: ingest.domain, source_url: ingest.source_url, ground_truth: ingest.ground_truth,
  case_text: caseText, segments: ingest.segments, coverage_ok,
  input_facts: inGraded, input_attempts: inputAttempts,
  leading_answer: derived.leading_answer, output_claims: outGraded, output_attempts: outputAttempts,
  input_rejected: inGraded.filter((g) => !g.grounded).length,
  output_rejected: outGraded.filter((g) => !g.grounded).length,
}
