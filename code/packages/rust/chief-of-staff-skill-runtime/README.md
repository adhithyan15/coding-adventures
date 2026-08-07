# chief-of-staff-skill-runtime

Provider-neutral runtime wrapper for D18 Level 1 `SKILL.md` agents. It sends
the parsed skill instructions and each verified channel message through the
repository `LlmClient`, publishes the text response, and acknowledges the input
only after publication succeeds.

`LevelOneSkillRuntime::from_verified_package` binds the exact instructions
retained by sealed-package verification. Loading rejects non-Skill runtimes and
packages whose signed manifest differs from the policy derived from the signed
`SKILL.md`.

The package performs no operating-system access. Channel endpoints and the LLM
provider are injected, so tests remain deterministic and deployments can swap
storage, transport, crypto, and model implementations independently.

## Validation

```sh
sh chief-of-staff-skill-runtime/BUILD
```
