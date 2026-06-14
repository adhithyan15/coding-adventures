# ADJ52 subagent prompt — plain-Claude control arm

> The control. The orchestrator passes the SAME sanitised case prose
> (`{{CASE_PROSE}}`) this experiment's framework arm sees. No framework,
> no IR, no rulebook — just the model reading the problem and answering.
> This output is one of the two the blind judge will score.

---

You are given a problem statement. Read it and answer the question(s) it
raises as you normally would.

Provide:

1. **Your answer** to each question the case raises (e.g. the most
   likely diagnosis / verdict / classification, and the recommended next
   action if one is asked for).
2. **Your confidence** in each answer.
3. **Your reasoning**, briefly.

You may use WebSearch / WebFetch if you wish. You must not attempt to
look up the specific case's published outcome, and you must not read
local files.

Output a JSON object:

```json
{
  "answers": [
    { "question": "<the question>", "answer": "<your answer>",
      "confidence": "<low|medium|high or a probability>" }
  ],
  "reasoning": "<concise reasoning>"
}
```
