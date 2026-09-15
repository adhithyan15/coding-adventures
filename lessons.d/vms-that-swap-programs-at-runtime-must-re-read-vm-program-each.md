---
category: Perl
---

# VMs that swap programs at runtime must re-read `$vm->{_program}` each step

— capturing the original code list once causes calls to loop in the caller after a context handler switches programs.
