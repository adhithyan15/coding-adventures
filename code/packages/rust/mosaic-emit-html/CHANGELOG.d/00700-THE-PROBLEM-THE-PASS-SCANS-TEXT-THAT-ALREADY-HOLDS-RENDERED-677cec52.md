- **The problem:** the pass scans text that already holds rendered loop rows.
  The security review showed that a marker spelled with only letters, `-` and
  `=` could be forged by host data such as a task name. The forged quote then
  moved later attributes out of their quotes, and in a Node reproduction an
  `onfocus` handler ran.
