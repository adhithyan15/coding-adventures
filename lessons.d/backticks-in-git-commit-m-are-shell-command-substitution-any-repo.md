# Backticks in `git commit -m "..."` are shell command substitution (any repo)

A commit message written with `-m` inside double quotes contained `` `review` `` and
`` `practises.knowledge` `` as inline code. zsh executed them:

    (eval):22: command not found: review
    (eval):22: command not found: practises.knowledge

The commit succeeded with those words **silently deleted** from the message —
"interleave a  lesson every three lessons" — and nothing failed. Found only by reading
the message back with `git log -1 --format=%B`.

**Write commit messages with a heredoc** (`git commit -F - <<'MSG'` … `MSG`), quoting
the delimiter so nothing expands. This applies to `gh pr create --body` too, which is
already conventionally written that way in this repo.
