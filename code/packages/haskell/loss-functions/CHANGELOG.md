# Changelog

## 0.1.0 - 2026-07-18

- Add mean squared, mean absolute, binary cross-entropy, and categorical
  cross-entropy losses.
- Add derivatives for every loss and clamp probability inputs away from zero
  and one.
- Reject empty and length-mismatched input vectors explicitly.
