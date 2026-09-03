//! Pins the SKILL.md-to-tool-surface contract: a document an author could
//! write, through the signed manifest shape, into a registered and invocable
//! tool.
//!
//! **This is not proof that production traverses this chain, and the first
//! draft of this file claimed it was.** A security review pointed out that
//! every link here has zero production callers -- `HostProfile::from_manifest`
//! (host-runtime's own doc comment says so), `ActiveHostToolRuntime`, and the
//! host-runtime crate itself, which `chief-of-staff-daemon` does not even
//! depend on. The daemon's shipped surface is `SmartHomeToolBridge` gated on
//! the hardcoded `PRODUCTION_SMART_HOME_MODEL_TOOLS` list; none of that is
//! exercised below.
//!
//! Writing a test to catch "boundaries on paths production never takes", and
//! then building it out of exactly such a path, is the fourth instance of that
//! mistake in this arc. Recorded rather than quietly corrected, because the
//! framing is what would make a green run misleading.
//!
//! Placed in `chief-of-staff-skill-package` rather than
//! `chief-of-staff-host-runtime` because this crate already depends on both
//! `skill-parser` and `host-runtime` in `[dependencies]`. The build tool's
//! `parseRustDeps` deliberately ignores `[dev-dependencies]`, and CI is
//! diff-based -- so as a host-runtime dev-dependency this test would NOT have
//! run on a PR that changed the parser, which is precisely the PR it exists to
//! catch.
//!
//! What it DOES pin is real: that the manifest schema, the SKILL.md sections,
//! the profile derivation and the registration checks agree with each other.
//! When the wiring lands, this is the contract it has to satisfy.

use chief_of_staff_host_runtime::{HostProfile, HostProfileRuntime, HostRuntimeError};
use chief_of_staff_skill_parser::parse_skill;
use chief_of_staff_tool_api::{
    builtin_tool_definition, PrivilegeTier, RequestedBy, ToolHandlerOutput, ToolInvocationRequest,
};
use coding_adventures_json_value::JsonValue;

/// A SKILL.md with nothing exotic in it: frontmatter, one capability, one
/// tool, one tool capability.
const SKILL: &str = "---\n\
agent: note-taker\n\
description: Records short notes into the session context for later recall.\n\
privilege_tier: 0\n\
reads: [note-requests]\n\
writes: [note-receipts]\n\
---\n\
# Note Taker\n\n\
Records short notes into the session context so they can be recalled later.\n\n\
## Capabilities needed\n\
- none\n\n\
## Tools needed\n\
- context.append_entry\n\n\
## Tool capabilities needed\n\
- context:write\n";

#[test]
fn a_skill_document_becomes_a_running_tool_surface() {
    // 1. The author's bytes parse into a signed-manifest-shaped contract.
    let skill = parse_skill(SKILL).expect("a well-formed SKILL.md parses");
    assert_eq!(skill.manifest.agent, "note-taker");
    assert_eq!(skill.manifest.allowed_tools, vec!["context.append_entry"]);
    assert_eq!(skill.manifest.tool_capabilities, vec!["context:write"]);

    // 2. The manifest renders to the canonical JSON a signer would sign, and
    //    re-parses identically. This is the byte-for-byte comparison
    //    `chief-of-staff-skill-package` performs against the signed copy, so
    //    if it fails here it fails at verification.
    let canonical = skill.manifest.to_json().expect("manifest renders");
    let reparsed = chief_of_staff_agent_manifest::parse_manifest(&canonical)
        .expect("canonical JSON re-parses");
    assert_eq!(reparsed, skill.manifest);

    // 3. The manifest derives the surface a supervisor enforces.
    let profile = HostProfile::from_manifest("orchestrator-1", &skill.manifest)
        .expect("a manifest with tools derives a profile");
    assert_eq!(profile.host_id, "note-taker");
    assert_eq!(profile.max_tier, PrivilegeTier::Tier0);
    assert_eq!(profile.allowed_tools, vec!["context.append_entry"]);
    assert_eq!(profile.capabilities, vec!["context:write"]);

    // 4. The declared tool registers -- meaning it cleared the tier ceiling,
    //    the capability check, the canonical-definition pin, and S-I7.
    let mut runtime = HostProfileRuntime::new(profile).expect("profile builds a runtime");
    let definition = builtin_tool_definition("context.append_entry")
        .expect("context.append_entry is a built-in");
    runtime
        .register_handler(definition, |_arguments, _context| {
            // Must satisfy the tool's declared OUTPUT schema too -- the first
            // draft of this test returned only `entry_id` and the runtime
            // refused it, which is output validation doing its job.
            Ok(ToolHandlerOutput::new(JsonValue::Object(vec![
                (
                    "session_id".to_string(),
                    JsonValue::String("sess-1".to_string()),
                ),
                (
                    "entry_id".to_string(),
                    JsonValue::String("entry-1".to_string()),
                ),
            ])))
        })
        .expect("a tool the manifest declares must register");

    // 5. Activation succeeds because every declared tool is wired. The
    //    refusal path is exercised separately below -- with one declared tool
    //    already registered, `missing_tools` here is empty trivially and the
    //    check could be deleted without this line noticing.
    let active = runtime
        .activate()
        .expect("every declared tool is registered");

    // 6. And the agent can call it. Note this runs under the default
    //    `AllowAllToolPolicy` and a stub handler, so it proves the dispatch
    //    path and the schemas agree -- not that any policy gate holds.
    let request = ToolInvocationRequest {
        call_id: "call-1".to_string(),
        tool_id: "context.append_entry".to_string(),
        arguments: JsonValue::Object(vec![
            (
                "session_id".to_string(),
                JsonValue::String("sess-1".to_string()),
            ),
            (
                "role".to_string(),
                JsonValue::String("assistant".to_string()),
            ),
            (
                "content".to_string(),
                JsonValue::String("remember the milk".to_string()),
            ),
        ]),
        requested_by: RequestedBy::Agent,
        session_id: None,
        job_id: None,
        agent_id: Some("note-taker".to_string()),
        user_id: None,
        requested_at: 0,
        deadline_at: None,
        idempotency_key: None,
    };
    let trace = active.invoke_with_events(&request);
    assert!(
        trace.result.ok,
        "the declared tool must actually run: {:?}",
        trace.result
    );
}

#[test]
fn a_tool_the_skill_did_not_declare_is_refused() {
    // The chain has to deny as well as permit, or step 4 above proves only
    // that registration is permissive.
    let skill = parse_skill(SKILL).expect("SKILL.md parses");
    let profile =
        HostProfile::from_manifest("orchestrator-1", &skill.manifest).expect("profile derives");
    let runtime = HostProfileRuntime::new(profile).expect("runtime builds");

    // Asserts the SPECIFIC variant. `memory.remember` is refused twice over --
    // absent from `allowed_tools` AND requiring `memory:write` the profile
    // lacks -- so a bare `is_err()` would stay green if the allow-list check
    // were deleted entirely, and this test's name would become a lie.
    let undeclared = builtin_tool_definition("memory.remember").expect("built-in exists");
    assert!(
        matches!(
            runtime.check_registration(&undeclared),
            Err(HostRuntimeError::ToolNotAllowed(ref id)) if id == "memory.remember"
        ),
        "must be refused for being undeclared, not for some other reason"
    );
}

#[test]
fn a_skill_declaring_a_tool_above_its_tier_cannot_register_it() {
    // `privilege_tier: 0` in the frontmatter is the ceiling, and `job.install`
    // is Tier1. The declaration alone must not be enough.
    let source = SKILL
        .replace("- context.append_entry", "- job.install")
        .replace("- context:write", "- jobs:install");
    let skill = parse_skill(&source).expect("SKILL.md parses");
    let profile =
        HostProfile::from_manifest("orchestrator-1", &skill.manifest).expect("profile derives");
    let runtime = HostProfileRuntime::new(profile).expect("runtime builds");

    // The fixture grants `jobs:install`, so the capability check would pass
    // and tier is the only thing left to fail on. Pinned to the variant so a
    // future S-I7 or capability refusal cannot quietly take credit.
    let above_tier = builtin_tool_definition("job.install").expect("built-in exists");
    assert!(
        matches!(
            runtime.check_registration(&above_tier),
            Err(HostRuntimeError::PrivilegeCeilingExceeded { .. })
        ),
        "must be refused on tier specifically"
    );
}

#[test]
fn a_partially_wired_surface_cannot_activate() {
    // The main test's `activate()` succeeds trivially: one declared tool, one
    // registered. Delete the CatalogIncomplete check and it stays green. This
    // declares two and wires one, so the refusal is actually exercised.
    let source = SKILL.replace(
        "- context.append_entry",
        "- context.append_entry\n- context.read_entries",
    );
    let skill = parse_skill(&source).expect("SKILL.md parses");
    assert_eq!(skill.manifest.allowed_tools.len(), 2);

    let profile =
        HostProfile::from_manifest("orchestrator-1", &skill.manifest).expect("profile derives");
    let mut runtime = HostProfileRuntime::new(profile).expect("runtime builds");
    runtime
        .register_handler(
            builtin_tool_definition("context.append_entry").expect("built-in exists"),
            |_arguments, _context| {
                Ok(ToolHandlerOutput::new(JsonValue::Object(vec![
                    (
                        "session_id".to_string(),
                        JsonValue::String("sess-1".to_string()),
                    ),
                    (
                        "entry_id".to_string(),
                        JsonValue::String("entry-1".to_string()),
                    ),
                ])))
            },
        )
        .expect("the first declared tool wires");

    // `context.read_entries` is declared and never registered.
    assert!(
        runtime.activate().is_err(),
        "a host whose declared tools are not all wired must not activate"
    );
}
