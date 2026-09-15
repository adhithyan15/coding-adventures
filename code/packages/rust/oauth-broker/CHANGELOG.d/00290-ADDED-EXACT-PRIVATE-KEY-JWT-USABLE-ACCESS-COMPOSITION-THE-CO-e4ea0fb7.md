- Added exact `private_key_jwt` usable-access composition: the complete retained
  provider/client/endpoint/method/algorithm/key profile is validated before
  credential or clock access, still-fresh tokens invoke no signer or transport,
  and due tokens cross audited assertion, transport, and revision-bound rotation
  before custody disclosure. Unused replay entropy remains zeroizing.
