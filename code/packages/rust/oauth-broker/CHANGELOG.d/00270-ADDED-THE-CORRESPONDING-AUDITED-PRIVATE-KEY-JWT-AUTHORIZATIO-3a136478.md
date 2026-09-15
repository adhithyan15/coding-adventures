- Added the corresponding audited `private_key_jwt` Authorization Code
  exchange-to-verified-identity custody composition. Exact identity bindings
  fail before signing; zeroizing ID-token evidence is consumed before storage;
  and no concrete signing or verification algorithm is enabled.
