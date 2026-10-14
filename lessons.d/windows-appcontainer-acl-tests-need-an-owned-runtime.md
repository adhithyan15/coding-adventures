# Windows AppContainer ACL tests need an owned runtime

A sandbox startup timeout was an earlier launcher exit: the overlap test tried to update the system Node executable DACL as a non-administrator. Copy the trusted runtime into an owned fixture, launch both children against that same executable, and keep the original attestation and case deadlines. Check launcher exit diagnostics before changing a timeout.

Run 38163286318 confirmed a separate timing failure after the owned-runtime fix:
the overlap took 2.3 seconds against a test-only two-second attestation limit,
and the launcher exited with cancellation code 1223 rather than DACL failure 66.
Use the production five-second startup budget and the Python overlap fixture's
five-second child lifetime. Both children wait for both launches to attest before
stdin closes, so the lifetime must allow that wait. Keep the 15-second case limit
and both successful-exit assertions; production enforcement stays unchanged.
