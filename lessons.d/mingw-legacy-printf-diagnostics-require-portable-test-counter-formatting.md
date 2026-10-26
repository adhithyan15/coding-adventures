---
category: Cross-platform & Windows BUILD_windows
---

# MinGW legacy printf diagnostics require portable test counter formatting

NN48's first strict local GCC compile failed in the test receipt, before any
numerical execution: Strawberry's MinGW printf diagnostics reject ISO C99 %zu
under -Werror. Use an explicit unsigned long long cast and %llu for bounded
test counts, as for the safe optimizer step receipt. Keep strict warnings;
do not classify this host CRT formatting issue as a training failure or claim
that the unexecuted suite passed. Rerun the actual suite after the repair.

The repaired GCC suite ran 526 checks with zero failures, including 3000 native
XOR updates and active all-parameter gradients for every activation.
