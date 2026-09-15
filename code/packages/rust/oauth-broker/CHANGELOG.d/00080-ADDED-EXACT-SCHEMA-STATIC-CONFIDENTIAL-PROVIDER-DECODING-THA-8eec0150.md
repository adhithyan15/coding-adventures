- Added exact-schema static confidential-provider decoding that retains one
  closed implemented authentication method while rejecting public `none`,
  absent, case-variant, and user-defined methods. JWT profiles require bounded,
  unique, non-`none` algorithm data while secret profiles reject algorithms;
  no credential/key access or concrete signer is added.
