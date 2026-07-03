export const meta = {
  name: 'adj58-crossdomain-judge',
  description: 'Blind judge for the cross-domain head-to-head. Per domain, reads two reports (A and B — one is plain Claude, one is the byte-provenance framework, order randomized + identity hidden) plus the held-aside ground truth, and decides which got the answer right and which reasoning is more trustworthy.',
  phases: [{ title: 'Judge' }],
}

const DOMAINS = ['engineering', 'astronomy', 'cybersecurity']
const DIR = '/Users/adhithya/Downloads/ca-crossdomain/code/specs/data/adj57/pipeline'

const SCHEMA = {
  type: 'object',
  required: ['winner', 'a_correct', 'b_correct', 'rationale'],
  properties: {
    winner: { type: 'string', enum: ['A', 'B', 'tie'] },
    a_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    b_correct: { type: 'string', enum: ['correct', 'partial', 'incorrect'] },
    more_trustworthy: { type: 'string', enum: ['A', 'B', 'equal'], description: 'which report is more defensible/auditable irrespective of who got the answer' },
    rationale: { type: 'string', description: 'specific: what each got right/wrong vs ground truth, and how trustworthy each reads' },
  },
}

const prompt = (domain) => `You are an impartial expert judge in ${domain}. Read the held-aside GROUND TRUTH and two reports (OUTPUT A, OUTPUT B) from systems whose identities are hidden — do NOT guess which is which. Score each report's CORRECTNESS against the ground truth (did it reach the right answer), its calibration (appropriate confidence), and its defensibility (is the reasoning traceable/auditable). Pick a winner on getting the answer right; also say which is more trustworthy.

Read the case + both reports here: ${DIR}/judge-${domain}.json

Return winner (A/B/tie on correctness), a_correct, b_correct (each correct/partial/incorrect vs ground truth), more_trustworthy, and a specific rationale.`

const results = await pipeline(
  DOMAINS,
  (domain) => agent(prompt(domain), { phase: 'Judge', label: `judge:${domain}`, agentType: 'general-purpose', schema: SCHEMA }).then((v) => ({ domain, ...v })).catch(() => null),
)
return { verdicts: results.filter(Boolean) }
