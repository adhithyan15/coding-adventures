# Windows AppContainer ACL tests need an owned runtime

A sandbox startup timeout was an earlier launcher exit: the overlap test tried to update the system Node executable DACL as a non-administrator. Copy the trusted runtime into an owned fixture, launch both children against that same executable, and keep the original attestation and case deadlines. Check launcher exit diagnostics before changing a timeout.
