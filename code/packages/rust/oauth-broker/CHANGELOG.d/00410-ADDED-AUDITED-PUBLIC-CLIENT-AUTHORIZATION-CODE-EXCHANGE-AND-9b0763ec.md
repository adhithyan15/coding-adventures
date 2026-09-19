- Added audited public-client Authorization Code exchange and exact-key credential creation.
  The broker now validates the caller-selected key, registered public `none`
  profile, client ID, and token endpoint before injected transport, clock, or
  credential custody. Bounded response decoding, credential release, and
  create-if-absent storage retain separate provider/trace audit gates, while
  only the opaque revision leaves the composition.
