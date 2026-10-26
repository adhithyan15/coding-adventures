---
category: Testing & coverage
---

# Windows Clang sanitizer binaries need the matching runtime DLL directory on PATH

NN47's Windows Clang ASan/UBSan executable compiled successfully but could not
start: exit -1073741515 (missing DLL) with no parser test output. LLVM's bin
directory was not enough. Its matching clang_rt.asan_dynamic-x86_64.dll lived
under lib/clang/22/lib/windows, so that exact runtime directory was required on
the child process PATH. With the matching runtime available, all checks passed.
Classify this as sanitizer bootstrap, not a source failure or successful test.
Never substitute a sanitizer DLL from another compiler release.
