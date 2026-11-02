## 0.1.1

- **Security fix.** `ct_select_bytes` built its mask with `choice ? 1u : 0u`, a
  source-level branch on the secret bit that CT01 forbids (despite the comment
  claiming otherwise). The mask now uses the value conversion
  `static_cast<unsigned>(choice)`.
