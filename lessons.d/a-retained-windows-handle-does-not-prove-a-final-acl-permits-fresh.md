---
category: Rust
---

# A retained Windows handle does not prove a final ACL permits fresh verification opens

A supported-policy probe retained READ_CONTROL granted at private creation,
so exact readback accepted a final DACL which denies fresh security opens.
Independent native review used an inheritable OWNER RIGHTS denial of
ReadPermissions: the second output passed probes, the first original changed,
then final verification failed. Rollback worked, but rejection came too late.

While the distinct permanently empty probe has the intended final policy,
reopen its active link with the same access mask as production verification.
Check identity, zero length and exact policy through that fresh handle before
restoring privacy. Retained handles prove object identity and setter capability;
they do not prove the final policy permits future opens. Keep a regression which
observes whether any original backup/removal boundary was reached.
