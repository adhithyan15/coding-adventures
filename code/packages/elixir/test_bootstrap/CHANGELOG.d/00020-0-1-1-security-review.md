## 0.1.1

- Security review fixes: the budget charge is now lock-free reserve-and-rollback
  (no spin lock, no `lock_spins` hook, no stuck state after a killed process);
  a closed or stale handle is spent instead of yielding a fresh root; `Canon`
  clips binaries and map reads before allocating and escapes invisible
  formatting characters; bare test receives have timeouts.
