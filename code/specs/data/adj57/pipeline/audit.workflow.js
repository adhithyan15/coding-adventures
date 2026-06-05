export const meta = {
  name: 'adj68-defensibility-audit',
  description: 'ADJ68 — open-book, audit-scored. The framework is NOT a recall engine; it produces auditable, defensible work. So: (1) spider+ground the facts a question needs, each in a verbatim source passage (CAS-style); (2) build the answer as a defensible CHAIN where every node cites a grounded fact, a cited rule, arithmetic, or a prior node; (3) an adversarial auditor tries to FAULT each chain, scoring VERIFIABILITY not correctness. Compared against a bare closed-book recall answer. A correct-but-unverifiable answer FAILS the audit; a grounded chain PASSES — measuring the axis the framework actually targets.',
  phases: [{ title: 'Ground' }, { title: 'ChainVsBare' }, { title: 'Audit' }],
}

const QUESTION = 'The thermal pericyclic cascade converting the heptaene into endiandric acid B methyl ester involves three steps: two electrocyclizations then a cycloaddition. Give each electrocyclization as [nπ]-con or [nπ]-dis (n = π electrons, con/dis = conrotatory/disrotatory) and the cycloaddition as [m+n] (atoms on each component).'

const GROUND_SCHEMA = {
  type: 'object', required: ['facts'],
  properties: {
    facts: {
      type: 'array',
      description: 'each fact grounded in a VERBATIM passage from a real fetched source.',
      items: {
        type: 'object', required: ['fact', 'kind', 'source_url', 'quote'],
        properties: {
          fact: { type: 'string' },
          kind: { type: 'string', enum: ['literature-fact', 'rule'], description: 'literature-fact = a structural fact about THIS reaction (e.g. the step order); rule = a general selection rule.' },
          source_url: { type: 'string' },
          quote: { type: 'string', description: 'verbatim passage from the source that establishes the fact' },
        },
      },
    },
  },
}

const CHAIN_SCHEMA = {
  type: 'object', required: ['answer', 'chain'],
  properties: {
    answer: { type: 'string', description: 'the final answer: step1, step2, step3' },
    chain: {
      type: 'array',
      description: 'the defensible reasoning chain; every node must be backed.',
      items: {
        type: 'object', required: ['claim', 'support'],
        properties: {
          claim: { type: 'string' },
          support: { type: 'string', description: 'one of: "grounded-fact: <source>", "rule: <source>", "arithmetic", "derivation from prior nodes" — what makes this claim checkable' },
        },
      },
    },
  },
}

const BARE_SCHEMA = {
  type: 'object', required: ['answer', 'reasoning'],
  properties: { answer: { type: 'string' }, reasoning: { type: 'string' } },
}

const AUDIT_SCHEMA = {
  type: 'object', required: ['audits'],
  properties: {
    audits: {
      type: 'array',
      items: {
        type: 'object', required: ['label', 'claims_total', 'claims_unsupported', 'unsupported_list', 'verdict', 'note'],
        properties: {
          label: { type: 'string' },
          claims_total: { type: 'integer' },
          claims_unsupported: { type: 'integer', description: 'claims asserted without a checkable citation or an explicitly-stated rule applied to a stated fact' },
          unsupported_list: { type: 'array', items: { type: 'string' } },
          verdict: { type: 'string', enum: ['PASS', 'FAIL'], description: 'PASS = a reader can verify EVERY link; FAIL = at least one unsupported assertion. Correctness does NOT count.' },
          note: { type: 'string' },
        },
      },
    },
  },
}

const groundPrompt = `Use web search and web fetch (open-book). Ground the facts needed to answer this chemistry question, each in a VERBATIM passage from a real source — do NOT assert from memory, cite.

QUESTION: ${QUESTION}

Ground at least these three:
  1. The ORDER of the two electrocyclizations in the Nicolaou endiandric-acid cascade — which π-electron count closes FIRST (8π or 6π). This is a structural fact about this specific reaction (kind="literature-fact").
  2. The Woodward–Hoffmann THERMAL selection rule for electrocyclizations: how 4n vs 4n+2 π electrons map to conrotatory vs disrotatory (kind="rule").
  3. That the final step is a Diels–Alder [4+2] cycloaddition (diene = 4 atoms, dienophile = 2 atoms) (kind="rule").
For each: the fact, its kind, the source_url you fetched, and a verbatim quote from it that establishes the fact.`

const chainPrompt = (facts) => `Construct the answer to this chemistry question as a DEFENSIBLE reasoning chain. You may use ONLY the grounded facts below plus arithmetic and explicit derivations — do NOT introduce any fact from your own memory; if you need something not grounded, you cannot use it.

QUESTION: ${QUESTION}

GROUNDED FACTS (the only external knowledge you may use):
${facts.map((f, i) => `  [G${i + 1}] (${f.kind}) ${f.fact}\n        source: ${f.source_url}\n        quote: "${(f.quote || '').slice(0, 160)}"`).join('\n')}

Build a chain of nodes. EACH node's claim must be backed by exactly one of: a grounded fact (cite "grounded-fact: <source>"), a rule (cite "rule: <source>"), "arithmetic", or "derivation from prior nodes". No unsupported leaps. End with the final answer (step1, step2, step3).`

const barePrompt = `You are taking a closed-book exam. Use ONLY your own knowledge — no tools, no web. Answer this question and give your reasoning.

QUESTION: ${QUESTION}`

const auditPrompt = (bare, chain) => `You are an adversarial AUDITOR. Two answers to the same chemistry question are below. For EACH, scrutinize every factual or inferential claim and mark it VERIFIABLE (backed by a checkable citation, OR an explicitly-stated rule applied to a stated fact, OR arithmetic) or UNSUPPORTED (asserted with no backing a reader could independently check — e.g. a fact stated from apparent memory, or a conclusion that does not follow from cited support).

Try to FAULT each. Give each an auditability VERDICT: PASS = a reader can verify EVERY link in the chain; FAIL = at least one claim is unsupported. CRUCIAL: do NOT reward correctness — a correct answer that a reader cannot verify step-by-step must FAIL. You are judging defensibility, not whether the answer is right.

--- Answer 1 ---
final: ${bare.answer}
reasoning: ${bare.reasoning}

--- Answer 2 ---
final: ${chain.answer}
chain:
${(chain.chain || []).map((n, i) => `  ${i + 1}. ${n.claim}  [support: ${n.support}]`).join('\n')}

Audit both (label them "Answer 1" and "Answer 2").`

// ---- run ----
const grounded = await agent(groundPrompt, { phase: 'Ground', label: 'spider-ground', agentType: 'general-purpose', schema: GROUND_SCHEMA })
const [chain, bare] = await parallel([
  () => agent(chainPrompt(grounded.facts), { phase: 'ChainVsBare', label: 'framework-chain', agentType: 'general-purpose', schema: CHAIN_SCHEMA }),
  () => agent(barePrompt, { phase: 'ChainVsBare', label: 'bare-recall', agentType: 'general-purpose', schema: BARE_SCHEMA }),
])
const audit = await agent(auditPrompt(bare, chain), { phase: 'Audit', label: 'adversarial-audit', agentType: 'general-purpose', schema: AUDIT_SCHEMA })

return {
  question: QUESTION,
  grounded_facts: grounded.facts,
  arm_A_bare: bare,
  arm_B_chain: chain,
  audit: audit.audits,
  ground_truth: '[8π]-con, [6π]-dis, [4+2] (order established: 8π conrotatory first, then 6π disrotatory; Nicolaou JACS 1982)',
}
