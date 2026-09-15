## Unreleased — McCarthy native host coverage (VM-036)

Run the existing McCarthy native capstone on Windows and Linux as well as
macOS, using each host's executable compiler and linker probe. A focused
19-program native corpus now runs in the Windows CI execution step; its
required-linker flag fails rather than reporting success without execution.

