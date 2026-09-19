- Added private-key-JWT static identity exchange custody. The broker validates
  the retained provider/client/endpoint/algorithm/key profile, opaque identity
  context, and nonce before loading exact static policy, then carries the
  response through the existing abstract signer, injected transport, audited
  identity verifier, and opaque credential custody. Only the revision is
  released, and no concrete signing or verification algorithm is enabled.
