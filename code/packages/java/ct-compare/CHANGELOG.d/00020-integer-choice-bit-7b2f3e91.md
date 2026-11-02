## 0.2.0

- **Security fix (breaking).** The selection `choice` is now an integer bit,
  exactly 0 (select `right`) or 1 (select `left`), instead of a boolean. This
  language has no branch-free conversion from a boolean to a number, so the old
  boolean API had to branch on the secret bit to build its mask, which CT01
  forbids. The mask is now `-choice & 0xFF`. Validation clears bit 0 and tests
  for zero, which takes the same path for both valid values; any other value is
  rejected before either input is read. This follows CT01's new "how a lane
  receives the selection bit" rule and Go's own `crypto/subtle`.
- Tests for rejected choice values and fresh-copy output.
