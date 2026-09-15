- Added exact client-secret usable-access composition: retained Basic/Post
  policy is validated before credential or clock access, still-fresh tokens
  reach only the audited custody closure, and due tokens cross the existing
  audited refresh plus revision-bound rotation path before disclosure. No
  secret or transport authority is invoked for a fresh token.
