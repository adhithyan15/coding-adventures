export const meta = {
  name: 'adj60-bidirectional-grounding',
  description: 'ADJ60 — prove BOTH directions of byte provenance on a fresh case. INPUT: decompose into a byte-covering partition (every input byte used or discarded-with-reason). OUTPUT: derive the answer as atomic CLAIMS each grounded_by verbatim input spans; an inline output-grounding gate rejects any claim whose citation is not verbatim in the input (smuggled from training) and KICKS IT BACK to re-derive, until every output claim traces to input bytes.',
  phases: [{ title: 'Ingest' }, { title: 'DeriveGrounded' }, { title: 'Kickback' }],
}

const INGEST_SCHEMA = {
  type: 'object',
  required: ['source_url', 'ground_truth', 'case_text', 'segments'],
  properties: {
    source_url: { type: 'string' },
    ground_truth: { type: 'string', description: 'the confirmed answer + how established. Held aside from later stages.' },
    case_text: { type: 'string', description: 'the EXACT scenario text decomposed (~500-1500 chars, observed evidence up to the decision point — NOT the named answer). segments MUST concatenate to this exactly.' },
    segments: {
      type: 'array',
      description: 'ordered partition of case_text; concatenating segment.text reproduces case_text character-for-character.',
      items: {
        type: 'object', required: ['text', 'kind'],
        properties: {
          text: { type: 'string' }, kind: { type: 'string', enum: ['fact', 'discard'] },
          term: { type: 'string' }, reason: { type: 'string' },
        },
      },
    },
  },
}
const DERIVE_SCHEMA = {
  type: 'object',
  required: ['leading_answer', 'claims'],
  properties: {
    leading_answer: { type: 'string', description: 'the answer — composed ONLY of what the claims below ground' },
    claims: {
      type: 'array',
      description: 'the atomic assertions that justify the answer. EVERY claim must be grounded_by verbatim input spans.',
      items: {
        type: 'object', required: ['claim', 'grounded_by'],
        properties: {
          claim: { type: 'string', description: 'one atomic assertion' },
          grounded_by: { type: 'array', items: { type: 'string' },
            description: 'one or more spans copied VERBATIM from the input (the case_text or a fact term) that support this claim' },
        },
      },
    },
  },
}

const ingestPrompt = `Find ONE real documented case (any identification/diagnosis domain — medicine, biology, geology, materials, forensics, etc.) and decompose its SCENARIO into a byte-covering IR.
1. Choose a self-contained scenario passage (~500-1500 chars) — the observed evidence up to the decision point. Do NOT include the named final answer. Copy VERBATIM into case_text.
2. Partition case_text into ordered segments; concatenating segment.text must reproduce case_text EXACTLY (every char). Each is a fact (snake_case term) or a discard (reason).
3. Return source_url and held-aside ground_truth.`

const deriveGroundedPrompt = (ingest) => `Answer this case under a strict OUTPUT-GROUNDING rule.

CASE TEXT:
${ingest.case_text}

EXTRACTED FACTS (verbatim terms you may cite):
${ingest.segments.filter((s) => s.kind === 'fact').map((s) => `  - ${s.term}`).join('\n')}

Emit your answer as: leading_answer (the conclusion) + claims (the atomic assertions that justify it). THE RULE: every claim must be grounded_by one or more spans copied VERBATIM from the CASE TEXT above (or an exact fact term). You may ONLY assert what the input bytes support — do NOT add a specific name, species, mechanism, or quantity that is not present in the input just because you recall it. If you cannot cite a verbatim input span for an assertion, do not make it. The leading_answer must contain nothing the claims do not ground.`

const kickbackPrompt = (ingest, prev, ungrounded) => `The OUTPUT-GROUNDING gate REJECTED these claims — their citations are NOT verbatim substrings of the input, so they assert something the input bytes do not support (likely recalled from training, not derived from the case):

${ungrounded.map((u) => `  - claim: ${u.claim}\n    (its grounded_by spans do not appear verbatim in the case text or facts)`).join('\n')}

Your previous leading_answer was: "${prev.leading_answer}".

For EACH rejected claim, you MUST either:
  (a) cite a span copied EXACTLY (verbatim, character-for-character) from the case text or a fact term that supports it, or
  (b) REMOVE the claim entirely (and remove any unsupported specificity it added to the leading_answer).
Re-emit the FULL corrected claims list and leading_answer. Every remaining claim must trace to verbatim input bytes. It is correct and expected to GENERALIZE the answer (drop specificity) when the input does not support the specific claim.`

// ---- run ----
const ingest = await agent(ingestPrompt, { phase: 'Ingest', label: 'ingest', agentType: 'general-purpose', schema: INGEST_SCHEMA })
const facts = ingest.segments.filter((s) => s.kind === 'fact').map((s) => s.term)
const hay = ingest.case_text + '\n' + facts.join('\n')
const ungroundedOf = (claims) => (claims || []).filter((c) => !((c.grounded_by || []).some((s) => s && hay.includes(s))))

let derived = await agent(deriveGroundedPrompt(ingest), { phase: 'DeriveGrounded', label: 'derive:attempt-1', agentType: 'general-purpose', schema: DERIVE_SCHEMA })
const attempts = [{ attempt: 1, leading_answer: derived.leading_answer, n_claims: derived.claims.length, n_ungrounded: ungroundedOf(derived.claims).length }]

for (let i = 2; i <= 4; i++) {
  const ung = ungroundedOf(derived.claims)
  if (ung.length === 0) break
  log(`output-grounding gate: ${ung.length} ungrounded claim(s) -> kicking back (attempt ${i})`)
  derived = await agent(kickbackPrompt(ingest, derived, ung), { phase: 'Kickback', label: `re-derive:attempt-${i}`, agentType: 'general-purpose', schema: DERIVE_SCHEMA })
  attempts.push({ attempt: i, leading_answer: derived.leading_answer, n_claims: derived.claims.length, n_ungrounded: ungroundedOf(derived.claims).length })
}

return { ingest, derived, attempts, final_ungrounded: ungroundedOf(derived.claims).length }
