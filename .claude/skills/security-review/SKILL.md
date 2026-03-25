---
description: >
  Run a security code review before pushing a PR. Launches a sub-agent
  that acts as a security expert for the language being used, reviews
  all changed code for vulnerabilities, and reports findings. The main
  agent then fixes issues and re-submits for review until the sub-agent
  confirms the code is clean. Only then does the push proceed.
  Trigger: "security review", "review before push", "check for vulnerabilities",
  or automatically before any git push when enabled in CLAUDE.md.
user_invocable: true
---

# Security Review

Launch a security-expert sub-agent to review all changed code before
pushing. Iterate on fixes until the review passes, then push.

## Instructions

### Step 1: Gather the diff

Determine what code is being pushed by running:

```
git diff origin/main...HEAD
```

If there's no diff (nothing to push), tell the user and stop.

Also detect the primary language(s) in the diff by looking at file extensions.

### Step 2: Launch the security review sub-agent

Use the `Agent` tool to launch a sub-agent with the following prompt.
Substitute `<LANGUAGE>` with the detected language(s) and `<DIFF>` with
the actual diff content.

**Sub-agent prompt:**

```
You are a senior security engineer specializing in <LANGUAGE>. You are
performing a security code review on a pull request diff.

Review the following code changes for security vulnerabilities. Focus on:

**General (all languages):**
- Injection attacks (SQL, command, LDAP, XPath, template injection)
- Cross-site scripting (XSS) — reflected, stored, and DOM-based
- Insecure deserialization
- Hardcoded secrets, API keys, tokens, or passwords
- Insecure cryptography (weak algorithms, poor key management)
- Path traversal and directory traversal
- Race conditions and TOCTOU bugs
- Improper error handling that leaks sensitive information
- Insecure random number generation
- Missing input validation or sanitization at trust boundaries
- Open redirects
- Unsafe file operations (symlink attacks, temp file issues)
- Denial of service vectors (ReDoS, algorithmic complexity attacks)
- Missing authentication or authorization checks

**Python-specific:**
- Unsafe use of pickle, eval, exec, or __import__
- SSRF via urllib/requests with user-controlled URLs
- Jinja2 template injection
- SQL injection via string formatting instead of parameterized queries
- Insecure yaml.load (use safe_load)
- subprocess with shell=True

**JavaScript/TypeScript-specific:**
- Prototype pollution
- eval() and Function() constructor abuse
- DOM manipulation with innerHTML/outerHTML
- Insecure postMessage handling
- npm dependency confusion risks
- Server-side request forgery

**Ruby-specific:**
- Mass assignment vulnerabilities
- Unsafe ERB/HAML rendering
- system() and backtick command injection
- Insecure YAML.load (use safe_load)
- Open redirect in redirects

**Go-specific:**
- Unsafe pointer operations
- Goroutine leaks
- Missing TLS verification
- Integer overflow/underflow
- Unchecked error returns that skip security-critical operations

**Rust-specific:**
- Unsafe blocks with inadequate justification
- Use-after-free in unsafe code
- Unchecked unwrap() on user-controlled input
- Missing bounds checks in unsafe code

**For each finding, report:**
1. **Severity**: CRITICAL / HIGH / MEDIUM / LOW / INFO
2. **File and line**: exact location in the diff
3. **Vulnerability type**: e.g., "SQL Injection", "Hardcoded Secret"
4. **Description**: what the vulnerability is and why it matters
5. **Suggested fix**: concrete code change to remediate

**If no security issues are found**, respond with exactly:
"SECURITY REVIEW PASSED — no vulnerabilities found."

**Important**: Only flag real security issues. Do not flag:
- Style issues, naming conventions, or formatting
- Missing documentation or comments
- Performance issues (unless they enable DoS)
- Code quality issues that have no security impact
- Test code that intentionally uses insecure patterns for testing

Here is the diff to review:

<DIFF>
```

### Step 3: Process the review results

After the sub-agent returns:

- **If "SECURITY REVIEW PASSED"**: Proceed to Step 5 (push).
- **If findings were reported**:
  1. Display the findings to the user with a summary
  2. For each CRITICAL or HIGH finding: fix it immediately
  3. For each MEDIUM finding: fix it unless ambiguous — if unsure, ask the user
  4. For LOW/INFO findings: report them to the user but don't block the push
  5. Commit fixes: `fix(security): <description of what was fixed>`
  6. Proceed to Step 4

### Step 4: Re-review

After fixing issues, gather the updated diff and launch the sub-agent
again with the new diff. Repeat Steps 2-3 until the sub-agent returns
"SECURITY REVIEW PASSED" or only LOW/INFO findings remain.

**Safety valve**: If you've done 3 rounds of review without converging,
stop and ask the user for guidance. Don't loop forever.

### Step 5: Push

Once the security review passes:

```
git push
```

Report to the user:
- That the security review passed
- How many rounds of review it took
- Summary of any fixes that were made
- Any LOW/INFO findings the user should be aware of

## Important notes

- Never skip the security review — if this skill is invoked, the review
  must complete before pushing
- The sub-agent is read-only — it reviews but does not modify code
- The main agent is responsible for all code fixes
- If the diff is very large (>5000 lines), split the review by file or
  directory to stay within context limits
- This skill replaces `git push` — do not push separately after invoking this
