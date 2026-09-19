- Added public-client profile binding before usable access or forced refresh.
  Both generic entry points now reject confidential registrations before
  credential, clock, transport, or token-use authority while retaining the
  exact stored record; existing audited public refresh and revision-bound
  rotation behavior remains unchanged.
