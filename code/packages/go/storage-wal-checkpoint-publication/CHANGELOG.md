# Changelog

- Added guarded initialization of the first Ready checkpoint after verifying both checkpoint slots and empty log/data reservations.
- Added three-step durable publication of WAL checkpoint successors.
