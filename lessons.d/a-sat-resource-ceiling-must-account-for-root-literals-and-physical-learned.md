---
category: Testing & coverage
---

# A SAT resource ceiling must account for root literals and physical learned-clause arena entries

A SAT port initially applied its literal quota to the main clause slice only.
Root units live in assignment state and learned reduction leaves arena records
and watcher IDs, so live-clause byte totals alone do not bound all retained
state. A security review produced a two-unit bypass and a repeated-reduction
arena-growth case. Track cumulative original literals separately from learned
stored literals, count both for storage checks, and impose a hard physical
arena-entry ceiling in addition to the live-literal limit. Builder preflight
must use the same cumulative quotas before emitting its first clause; parser
overrides must also stay within literal representability. Test every limit
with small configured values and assert refused operations leave vars, clauses,
and counters unchanged.
