- Added exact stored-refresh-token detach through retained `private_key_jwt`:
  account, client, revocation audience, and algorithm bindings are checked
  before credential access; signing and transport remain injected and audited;
  and only exact HTTP 200 permits revision-bound local deletion.
