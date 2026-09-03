//! The whole chain, end to end: a SKILL.md an author could actually write,
//! through the signed manifest contract, into a running tool surface.
//!
//! Every link here was built separately over the last several changes --
//! manifest schema v3 (`allowed_tools`) and v4 (`tool_capabilities`), the
//! SKILL.md sections that populate them, `HostProfile::from_manifest`, and the
//! D18S S-I7 registration gate. Each landed with unit tests proving its own
//! half worked. None of them proved the chain worked.
//!
//! That distinction is the reason this file exists. A stack whose every link
//! passes its own tests can still be a stack nothing traverses, and this repo
//! has already produced three boundaries sitting on paths production never
//! takes. So this test starts from bytes an author writes and ends at a tool
//! result, touching nothing else, and it fails if any link in between stops
//! carrying its neighbour.

use chief_of_staff_host_runtime::{HostProfile, HostProfileRuntime};
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

    // 5. Activation refuses a host whose declared tools were not all wired, so
    //    reaching here means the surface is complete rather than partial.
    let active = runtime
        .activate()
        .expect("every declared tool is registered");

    // 6. And the agent can actually call it.
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

    let undeclared = builtin_tool_definition("memory.remember").expect("built-in exists");
    assert!(
        runtime.check_registration(&undeclared).is_err(),
        "a tool absent from the SKILL.md must not register"
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

    let above_tier = builtin_tool_definition("job.install").expect("built-in exists");
    assert!(
        runtime.check_registration(&above_tier).is_err(),
        "a Tier1 tool must not register on a Tier0 agent"
    );
}
