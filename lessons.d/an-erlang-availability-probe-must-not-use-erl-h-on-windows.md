# An Erlang availability probe must not use `erl -h` on Windows

Provisioning BEAM on the Windows build lane exposed a latent hang in
`twig-to-beam`: its runtime integration test used `erl -h` to decide whether
Erlang was installed. On Windows, that command starts an interactive shell and
does not exit, so the repository build remained stuck until the 150-minute CI
step timeout. Use a bounded no-shell command such as
`erl -noshell -eval "halt()."` for availability checks, and normalize Windows
backslashes to forward slashes before embedding a filesystem path in an Erlang
string literal.
