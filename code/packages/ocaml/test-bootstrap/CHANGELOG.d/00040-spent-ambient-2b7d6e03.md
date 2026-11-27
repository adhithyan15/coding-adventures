## 0.1.0 (security review, round 2)

- A domain that inherited an ambient scope whose whole chain has since closed now gets the SPENT
  node from `Budget.current ()`, so a plain `check` there is property-exhausted (never a fresh root:
  no cap escape for a leaked worker domain; matches the Elixir lane). A closed scope with an open
  ancestor still resolves to that ancestor.
- A scope's `finally` restores `skip_closed saved`, never a closed node.
- Documented: concurrent checks in fibres or threads of one domain without `?parent` may be charged
  to another check's tree and get property-exhausted (fail-closed).
