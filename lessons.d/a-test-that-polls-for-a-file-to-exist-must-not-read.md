---
category: Testing & coverage
---

# A test that polls for a file to exist must not read it before the writer renames it into place; fs::write creates before it fills

`chief-of-staff-broker-e2e`'s scripted host wrote its result with
`std::fs::write`, and the test waited with `result.is_file()` before reading
it. `fs::write` opens (creating and truncating) and only then writes, so on
a slow Ubuntu runner the test read the file in between and saw `""`:
`a_reservation_left_by_a_dead_broker_is_abandoned_at_launch` failed with an
empty assertion message at `supervised_broker.rs:469`.

Fix: the writer writes to a sibling `*.partial` file and `rename`s it onto
the result path. A rename within one directory is atomic, so the result is
either absent or whole.

Do differently: whenever one process signals another by a file appearing,
publish the file by rename (or poll for its full content, never its
existence). An empty panic message after `{contents}` is the tell.
