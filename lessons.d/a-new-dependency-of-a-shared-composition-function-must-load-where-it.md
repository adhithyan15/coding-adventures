---
category: Rust
---

# A new dependency of a shared composition function must load where it is used, not at the top

**What went wrong.** P1.4c gave the daemon's data-plane composition a new
input: the package keyring and the loaded vault, which the agent tool source
needs. My first version loaded both at the top of `compose_host_data_plane`
and the test-only `compose_data_plane_service`. Three existing tests broke:

- Two ran a production composition from a config whose trusted key file did
  not exist. They failed with `Keyring(KeyFileUnavailable)`.
- One expected `Authority(_)` from a bad channel key. It got the keyring
  error first, because the new load ran ahead of the step it was testing.

**The fix.** Load the inputs at the one point a model-tool surface is
actually built. `AgentToolInputs::resolve(None, ..)` runs there, after
authority and grant provisioning. A data plane with no models touches
neither the keyring nor the vault, and the earlier failures keep reporting
first. The two real production-composition tests now write the key file
their config names, as a real deployment has it.

**Do differently.** When a composition function used by many callers gains
an input, find where that input is first needed and load it there. Loading
it at the top changes the error every existing caller sees, even callers
that never reach the new code.
