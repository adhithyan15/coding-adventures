# Autonomous loop must self-heal: a single ScheduleWakeup is not durable

The SIR-completion loop used `ScheduleWakeup(180s)` to babysit each PR. When the
session went idle/closed, the wakeup chain stopped and PR #6561 (M3) sat
unbabysat for ~3 days (the user had to nudge it back). LESSON: a session-scoped
ScheduleWakeup loop does NOT survive a closed session — for multi-day autonomy,
prefer a durable cloud schedule (the `/schedule` skill / CronCreate), and on
EVERY wake re-derive state from `git fetch` + `gh pr list` rather than assuming
the previous turn's plan still holds. Also: a merged PR with no open follow-up
means "pick up the next item," not "idle" — check `mergedAt`, mark the task
done, and immediately start the next backlog item in the same turn.
