# Pinned qpdf for the Windows PDF oracle

The Rust PDF test suite uses qpdf as an independent structural oracle. The
Windows CI job must install and run that oracle; a missing executable is a hard
failure, not a skipped test.

## Windows artifact

Use the qpdf 12.4.2 x64 MSVC ZIP published in the qpdf GitHub release:

`https://github.com/qpdf/qpdf/releases/download/v12.4.2/qpdf-12.4.2-msvc64.zip`

Pin its SHA-256 to
`db87077e683630c1217e0e8f9a20a9749d952ab676e881c3689187763a5de25d`, the
digest published for this asset by the GitHub release API. Download it into
`RUNNER_TEMP`, verify the digest before extracting, and extract into a
dedicated temporary directory. The executable is at
`qpdf-12.4.2-msvc64/bin/qpdf.exe` inside the extracted directory.

Add that exact `bin` directory to both the current Bash process's `PATH` and
the GitHub Actions path for later steps. Invoke qpdf's version command and
require the output to identify version 12.4.2 before the PDF tests run. A
download, checksum, extraction, path, or version failure must fail the job.

This Windows acquisition is independent of the Linux apt and macOS Homebrew
install paths. It must preserve the `qpdf --check` oracle tests and must not
turn a missing oracle into a skip or a success. The fixed release URL and
digest make the executable bytes reproducible even if the hosted runner image
or package-manager feed changes.
