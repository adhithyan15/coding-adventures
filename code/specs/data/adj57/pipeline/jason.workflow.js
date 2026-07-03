export const meta = {
  name: 'adj-jason-genealogy',
  description: 'Open-book defensibility run on a contested genealogy question: "who was Jason\'s maternal great-grandfather?" The framework spiders + grounds each link of the maternal lineage in a verbatim source, EXPLICITLY surfacing (a) the definitional ambiguity of "maternal great-grandfather" and (b) the source disagreement over Jason\'s mother — vs a bare closed-book recall answer. An adversarial auditor scores whether every link is sourced AND whether the ambiguities are surfaced rather than papered over.',
  phases: [{ title: 'Ground' }, { title: 'ChainVsBare' }, { title: 'Audit' }],
}

const QUESTION = 'In Greek mythology, who was Jason\'s (the Argonaut\'s) maternal great-grandfather?'

const GROUND_SCHEMA = {
  type: 'object',
  required: ['interpretation_note', 'links', 'answer'],
  properties: {
    interpretation_note: { type: 'string', description: 'note the definitional ambiguity: "maternal great-grandfather" could be the father of the maternal GRANDFATHER or of the maternal GRANDMOTHER — state which line(s) you trace.' },
    links: {
      type: 'array',
      description: 'each genealogical link, grounded in a verbatim source quote. Mark contested links where ancient sources disagree.',
      items: {
        type: 'object', required: ['claim', 'source_url', 'quote', 'contested'],
        properties: {
          claim: { type: 'string', description: 'e.g. "Jason\'s mother is Alcimede" or "Phylacus is the son of Deion"' },
          source_url: { type: 'string' },
          quote: { type: 'string', description: 'verbatim passage establishing the link' },
          contested: { type: 'boolean', description: 'true if ancient sources disagree on this link' },
          note: { type: 'string', description: 'for contested links: the competing variants + their sources' },
        },
      },
    },
    answer: { type: 'string', description: 'the defensible answer: name the branch points and the maternal great-grandfather under EACH tradition; do NOT collapse a contested lineage into a single name.' },
  },
}

const BARE_SCHEMA = { type: 'object', required: ['answer', 'reasoning'], properties: { answer: { type: 'string' }, reasoning: { type: 'string' } } }

const AUDIT_SCHEMA = {
  type: 'object', required: ['audits'],
  properties: {
    audits: {
      type: 'array',
      items: {
        type: 'object', required: ['label', 'links_total', 'links_unsupported', 'surfaces_source_disagreement', 'surfaces_definitional_ambiguity', 'verdict', 'note'],
        properties: {
          label: { type: 'string' },
          links_total: { type: 'integer' },
          links_unsupported: { type: 'integer', description: 'genealogical links asserted with no checkable citation' },
          surfaces_source_disagreement: { type: 'boolean', description: 'does it surface that ancient sources disagree on Jason\'s mother, rather than papering over it?' },
          surfaces_definitional_ambiguity: { type: 'boolean', description: 'does it note the maternal-great-grandfather definitional ambiguity?' },
          verdict: { type: 'string', enum: ['DEFENSIBLE', 'NOT_DEFENSIBLE'], description: 'DEFENSIBLE = every link sourced AND ambiguities surfaced; NOT_DEFENSIBLE otherwise. Correctness of a single name is NOT the criterion.' },
          note: { type: 'string' },
        },
      },
    },
  },
}

const groundPrompt = `Use web search and web fetch (open-book). Answer this question with FULL traceability — every genealogical link grounded in a verbatim source, and every ambiguity surfaced, not hidden.

QUESTION: ${QUESTION}

1. State the DEFINITIONAL ambiguity of "maternal great-grandfather" (father of the maternal grandfather vs father of the maternal grandmother) and which line(s) you will trace.
2. Trace Jason's maternal lineage. Ground EACH link (Jason's mother; her father = maternal grandfather; his father = maternal great-grandfather) in a verbatim quote from a real source (Apollonius' Argonautica, Apollodorus' Bibliotheca, Hyginus, Tzetzes, theoi.com, etc.).
3. CRUCIAL: ancient sources DISAGREE on who Jason's mother was. Do NOT pick one silently — mark that link contested and list the competing traditions (e.g. Alcimede daughter of Phylacus vs Polymede daughter of Autolycus) each with its source, and carry each forward to its own great-grandfather.
4. Give a defensible answer that names the branch points and the maternal great-grandfather UNDER EACH tradition. Do not collapse a contested lineage into a single confident name.`

const barePrompt = `You are taking a closed-book quiz. Use ONLY your own knowledge — no tools, no web. Answer concisely and give brief reasoning.

QUESTION: ${QUESTION}`

const auditPrompt = (bare, ground) => `You are an adversarial AUDITOR judging DEFENSIBILITY, not whether a single name is "right". Two answers to a contested mythological-genealogy question are below.

For EACH answer assess: (a) is every genealogical link backed by a checkable cited source? count unsupported links; (b) does it SURFACE that ancient sources disagree on Jason's mother (vs papering it over with one confident name)? (c) does it note the "maternal great-grandfather" definitional ambiguity? Verdict DEFENSIBLE only if every link is sourced AND both ambiguities are surfaced.

--- Answer 1 (bare recall) ---
answer: ${bare.answer}
reasoning: ${bare.reasoning}

--- Answer 2 (grounded trace) ---
interpretation: ${ground.interpretation_note}
links:
${(ground.links || []).map((l, i) => `  ${i + 1}. ${l.claim}  [contested=${l.contested}]  src: ${l.source_url}`).join('\n')}
answer: ${ground.answer}

Audit both (label "Answer 1" and "Answer 2").`

// ---- run ----
const ground = await agent(groundPrompt, { phase: 'Ground', label: 'spider-genealogy', agentType: 'general-purpose', schema: GROUND_SCHEMA })
const bare = await agent(barePrompt, { phase: 'ChainVsBare', label: 'bare-recall', agentType: 'general-purpose', schema: BARE_SCHEMA })
const audit = await agent(auditPrompt(bare, ground), { phase: 'Audit', label: 'adversarial-audit', agentType: 'general-purpose', schema: AUDIT_SCHEMA })

return { question: QUESTION, grounded: ground, bare_recall: bare, audit: audit.audits }
