export const meta = {
  name: 'e2-localize',
  description: 'E2 correctability — blind-auditor localization, framework vs plain, on matched both-arms-wrong solutions. Per cell: (1) a BLIND Opus auditor (no gold, format-normalized envelope only) names the most-likely-flawed step; (2) a gold-aware Opus ORACLE+SCORE establishes the true error locus for that solution and judges the blind audit hit/partial/miss and whether the auditor was fooled. Same auditor on both arms, so the delta is the artifact. Rate-limit-safe batches of 10 cells (each cell = audit -> score chain).',
  phases: [{ title: 'BlindAudit', detail: 'no gold, normalized envelope' }, { title: 'OracleScore', detail: 'gold-aware locus + scoring' }],
}

const CELLS = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/e2-correctability/cells'
const ORACLE = '/Users/adhithya/Downloads/coding-adventures/code/specs/data/e2-correctability/oracle'
const ids = Array.isArray(args) ? args : (typeof args === 'string' ? JSON.parse(args) : args)
const BATCH = 10

const AUDIT_SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['found_flaw', 'flaw_location', 'flaw_desc'],
  properties: {
    found_flaw: { type: 'boolean', description: 'true if you can point to a specific flawed step/quantity' },
    flaw_location: { type: 'string', description: 'the single most-likely-flawed step/quantity, or "could not localize"' },
    flaw_desc: { type: 'string', description: '<= 40 words: why it is wrong' },
  },
}
const SCORE_SCHEMA = {
  type: 'object', additionalProperties: false,
  required: ['error_locus', 'localization', 'auditor_fooled'],
  properties: {
    error_locus: { type: 'string', description: 'the TRUE step/quantity where this solution first diverges from correct' },
    localization: { type: 'string', enum: ['hit', 'partial', 'miss', 'n/a'], description: 'did the blind audit localize the SAME error? n/a only if the solution is actually correct' },
    auditor_fooled: { type: 'boolean', description: 'true if the blind auditor declared the (wrong) solution sound OR affirmed a false load-bearing premise as correct' },
    note: { type: 'string', description: '<= 30 words' },
  },
}

function cell(cid) {
  const cpath = `${CELLS}/cell_${String(cid).padStart(4, '0')}.json`
  const opath = `${ORACLE}/cell_${String(cid).padStart(4, '0')}.json`
  const auditPrompt =
    `You are a domain-competent reviewer auditing a solution for errors. You do NOT have the ` +
    `answer key. Read the JSON at this path with the Read tool:\n${cpath}\nIt has { question, ` +
    `normalized } where "normalized" is the solution to audit.\n\nYour job: find WHERE this ` +
    `specific reasoning goes wrong by checking its logic, arithmetic, and stated facts — point to ` +
    `the SINGLE most-likely-flawed step/quantity. Do NOT re-derive the whole problem from scratch; ` +
    `audit what is written. If you genuinely cannot localize a flaw, set found_flaw=false and ` +
    `flaw_location="could not localize". Return the structured audit.`
  return agent(auditPrompt, { label: `audit:${cid}`, phase: 'BlindAudit', schema: AUDIT_SCHEMA })
    .then((audit) => {
      if (!audit) return null
      const scorePrompt =
        `You are an ORACLE with the answer key. Read the JSON at this path with the Read tool:\n` +
        `${opath}\nIt has { question, gold, answer, reasoning_raw } — the gold answer and a ` +
        `solution's actual reasoning.\n\nStep 1: establish the TRUE error_locus — the specific ` +
        `step/quantity where this solution first diverges from correct (or "n/a" if the solution's ` +
        `answer actually matches gold).\n\nStep 2: a BLIND auditor (no answer key) tried to ` +
        `localize the flaw and reported:\n  found_flaw=${audit.found_flaw}\n  location: ` +
        `${audit.flaw_location}\n  why: ${audit.flaw_desc}\nJudge whether the blind audit localized ` +
        `the SAME error you found: "hit" (same step/error), "partial" (right area, imprecise), ` +
        `"miss" (wrong or no localization). Set auditor_fooled=true if the blind auditor instead ` +
        `declared this wrong solution sound, or affirmed a false load-bearing premise as correct. ` +
        `Return the structured score.`
      return agent(scorePrompt, { label: `score:${cid}`, phase: 'OracleScore', schema: SCORE_SCHEMA })
        .then((score) => (score ? { cid, audit, score } : null))
    })
}

const all = []
for (let i = 0; i < ids.length; i += BATCH) {
  const batch = ids.slice(i, i + BATCH)
  log(`Batch ${i / BATCH + 1}: cells ${batch[0]}..${batch[batch.length - 1]} (${all.length} done).`)
  const res = await parallel(batch.map((cid) => () => cell(cid)))
  all.push(...res.filter(Boolean))
}
log(`Collected ${all.length}/${ids.length} localize results.`)
return all
