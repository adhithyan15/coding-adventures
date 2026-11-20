category: Testing & coverage
---

# Assert a negative-control mutation changes its intended input

A digest negative control assigned the same role value already present in the
selected registry row, so the supposedly mutated payload was unchanged. A
passing comparison then proved nothing about sensitivity. Choose a mutation
relative to the concrete baseline and assert that the input bytes or parsed
payload actually changed before checking that the digest changes.
