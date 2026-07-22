use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use capability_os_sandbox::{current_kernel_sandbox_support, OsFamily};
use chief_of_staff_tool_api::{
    ApprovalAssurance, ApprovalState, PrivilegeTier, ToolAuditRecordQuery,
};
use chief_of_staff_tool_audit_store::ToolAuditStore;
use os_job_core::BackendKind;
use storage_local_folder::LocalFolderStorageBackend;
use weather_agent_e2e::{
    run_umbrella_today_agent, RecommendationKind, UmbrellaAgentConfig, WeatherFetchSourceKind,
};

#[test]
fn umbrella_today_agent_exercises_architecture_and_writes_text_file() {
    let root = temp_root("weather-agent-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");

    let run = run_umbrella_today_agent(UmbrellaAgentConfig::deterministic_seattle(&output_path))
        .expect("umbrella E2E should complete");

    assert_eq!(run.recommendation.kind, RecommendationKind::Both);
    assert!(run.recommendation.kind.needs_umbrella());
    assert_eq!(run.output_path, output_path);
    assert!(run.output_text.contains("Bring an umbrella today"));
    assert!(run.output_text.contains("precipitation chance is 72%"));
    assert!(output_path.exists());

    assert_eq!(
        run.weather_fetch.source_kind,
        WeatherFetchSourceKind::Fixture
    );
    assert_eq!(run.weather_fetch.https_requests, 0);
    assert_eq!(run.weather_fetch.tls_handshakes, 0);
    assert_eq!(run.weather_fetch.http_statuses, vec![200]);
    assert!(!run.weather_fetch.http_domain_policy_enforced);
    assert!(run.weather_fetch.declared_http_domains.is_empty());

    assert_eq!(run.sandbox_plan.package, "rust/weather-agent-e2e");
    assert_eq!(run.sandbox_plan.plan_count, 6);
    assert_eq!(run.sandbox_plan.total_rules, 18);
    assert_eq!(run.sandbox_plan.current_os, OsFamily::current());
    assert_eq!(run.sandbox_plan.current_os_rules, 3);
    assert!(run.sandbox_plan.direct_rules >= 8);
    assert!(run.sandbox_plan.brokered_rules >= 4);
    assert!(run.sandbox_plan.native_rules >= 12);
    assert_eq!(
        run.sandbox_plan
            .summary_for_os(OsFamily::Linux)
            .expect("linux plan should be present")
            .total_rules,
        3
    );
    assert_eq!(
        run.sandbox_plan
            .summary_for_os(OsFamily::Portable)
            .expect("portable plan should be present")
            .host_broker_rules,
        3
    );

    let kernel_support = current_kernel_sandbox_support();
    assert_eq!(run.kernel_sandbox.os, OsFamily::current());
    assert_eq!(run.kernel_sandbox.available, kernel_support.available);
    if kernel_support.available {
        assert!(run.kernel_sandbox.enforced);
        assert!(run.kernel_sandbox.allowed_write_succeeded);
        assert!(run.kernel_sandbox.denied_write_blocked);
        assert!(run.kernel_sandbox.network_outbound_enforced);
        assert!(run.kernel_sandbox.network_denied_outbound_blocked);
        assert!(run
            .kernel_sandbox
            .denied_network_target
            .starts_with("localhost:"));
        assert!(!run.kernel_sandbox.weather_https_allowed);
        assert!(!run.kernel_sandbox.host_exact_kernel_enforced);
        assert!(run.kernel_sandbox.stderr_contains_operation_not_permitted);
        assert!(!run.kernel_sandbox.denied_path.exists());
    }

    assert_eq!(run.supervisor.child_count, 3);
    assert!(run.orchestrator_profile.active);
    assert_eq!(run.orchestrator_profile.profile_id, "umbrella_today_v1");
    assert_eq!(run.orchestrator_profile.host_count, 4);
    assert_eq!(run.orchestrator_profile.allowed_tool_count, 4);
    assert_eq!(run.orchestrator_profile.registered_tool_count, 4);
    assert_eq!(run.supervisor.stopped_children, 3);
    assert_eq!(run.supervisor.failed_children, 0);
    assert_eq!(run.supervisor.dead_letters, 0);
    assert_eq!(run.supervisor.messages_processed, 3);
    assert_eq!(run.supervisor.restart_count, 0);
    assert_eq!(run.actor_channel_messages, 3);

    assert_eq!(run.tool_journal_health.invocation_count, 4);
    assert_eq!(run.tool_journal_health.completed_count, 4);
    assert_eq!(run.tool_journal_health.failed_count, 0);
    assert_eq!(run.tool_journal_health.approval_granted_count, 2);
    assert_eq!(run.tool_journal_health.approval_pending_count, 0);
    assert!(run.tool_journal_health.results_with_output_count >= 4);
    assert!(run.tool_journal_health.results_with_artifact_refs_count >= 1);

    assert_eq!(run.context_inventory.sessions.total_sessions, 1);
    assert_eq!(run.context_inventory.transcripts.user_entries, 1);
    assert_eq!(run.context_inventory.transcripts.tool_call_entries, 4);
    assert_eq!(run.context_inventory.transcripts.tool_result_entries, 4);
    assert_eq!(run.context_inventory.transcripts.assistant_entries, 1);
    assert_eq!(run.context_inventory.snapshots.total_snapshots, 1);
    assert!(run.context_inventory.sessions_with_tool_activity >= 1);

    assert_eq!(run.artifact_inventory.total_artifacts(), 1);
    assert_eq!(run.memory_inventory.total_memories(), 2);
    assert_eq!(run.skill_inventory.total_skill_versions(), 1);

    assert_eq!(run.job_plan_backend, BackendKind::InProcess);
    assert_eq!(run.job_plan_file_count, 0);
    assert!(run.job_executor_status.is_quiescent());
    assert!(run.job_executor_status.has_executors());
    assert!(run.job_receipt.exit_status.is_success());
    assert_eq!(run.job_receipt.job_id, "umbrella_today_job");
    assert_eq!(
        run.job_receipt.output_refs,
        vec!["artifact:umbrella_today_report"]
    );
    assert_eq!(run.user_report.headline, "Bring an umbrella today");
    assert!(run
        .user_report
        .detail
        .contains("precipitation chance is 72%"));
    assert_eq!(run.user_report.write_approval, ApprovalState::Granted);
    assert_eq!(run.user_report.write_required_tier, PrivilegeTier::Tier1);
    assert_eq!(
        run.user_report.approval_assurance,
        Some(ApprovalAssurance::ExplicitConsent)
    );
    assert_eq!(run.user_report.journal_invocation_count, 4);
    assert!(run
        .vault_lease
        .vault_ref
        .as_str()
        .starts_with("vault-lease:"));
    assert_eq!(run.vault_lease.ttl_ms, 30_000);
    assert!(run.vault_lease.consumed_by_host);
    assert!(!run
        .vault_lease
        .vault_ref
        .as_str()
        .contains("host-only-fixture"));
    assert!(!run
        .output_text
        .contains("weather-api-key-host-only-fixture"));
    assert!(run.user_report.render().contains("Bring an umbrella today"));
    assert_eq!(run.durable_audit.job_id, "umbrella_today_job");
    assert_eq!(run.durable_audit.run_id, run.job_receipt.run_id);
    assert_eq!(run.durable_audit.session_id, "umbrella_today_session");
    assert_eq!(run.durable_audit.user_id, "seattle_user");
    assert_eq!(run.durable_audit.profile_id, "umbrella_today_v1");
    assert_eq!(run.durable_audit.host_count, 4);
    assert_eq!(run.durable_audit.tool_count, 4);
    assert_eq!(run.durable_audit.persisted_records, 4);
    assert_eq!(run.durable_audit.reloaded_records, 4);
    assert_eq!(run.durable_audit.completed_records, 4);
    assert_eq!(run.durable_audit.approval_granted_records, 2);
    assert!(run.durable_audit.records_with_references >= 1);
    assert_eq!(run.durable_audit.follow_up_records, 0);
    assert!(run.durable_audit.is_complete());

    let restarted_audit = ToolAuditStore::new(LocalFolderStorageBackend::new(
        output_path.with_extension("audit"),
    ));
    let restarted_rows = restarted_audit
        .query_audits(&ToolAuditRecordQuery::new())
        .expect("durable audit should reopen after the job runtime is dropped");
    assert_eq!(restarted_rows.len(), 4);
    assert!(restarted_rows
        .iter()
        .any(|row| row.tool_id == "vault.request_lease"));
    assert!(!format!("{restarted_rows:?}").contains("weather-api-key-host-only-fixture"));

    assert_eq!(run.rws.fetcher.untrusted_inputs, 1);
    assert_eq!(run.rws.writer.external_actuations, 1);
    assert!(run.rws.combined_manifest_rejected);
}

#[test]
fn umbrella_today_job_blocks_write_without_explicit_approval() {
    let root = temp_root("weather-agent-approval-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");
    let config = UmbrellaAgentConfig::deterministic_seattle(&output_path).without_write_approval();

    let error = run_umbrella_today_agent(config)
        .expect_err("write step should stop at the centralized approval gate");
    assert!(error.to_string().contains("requires approval"));
    let persisted_text = fs::read_to_string(&output_path).unwrap_or_default();
    if current_kernel_sandbox_support().available {
        assert_eq!(persisted_text, "kernel sandbox allowed");
    }
    assert!(!persisted_text.contains("Bring an umbrella today"));

    let restarted_audit = ToolAuditStore::new(LocalFolderStorageBackend::new(
        output_path.with_extension("audit"),
    ));
    let denied_write = restarted_audit
        .fetch_audit("8a7b0000000000000000000000000001_write_umbrella_report")
        .expect("durable audit should be readable after denial")
        .expect("denied write row should be persisted");
    assert_eq!(denied_write.approval_state, ApprovalState::Pending);
    assert!(!denied_write.result_summary.ok);
}

#[test]
fn umbrella_today_job_blocks_vault_lease_without_tier2_approval() {
    let root = temp_root("weather-agent-vault-approval-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");
    let config = UmbrellaAgentConfig::deterministic_seattle(&output_path).without_vault_approval();

    let error = run_umbrella_today_agent(config)
        .expect_err("vault lease should stop at the centralized Tier 2 approval gate");
    assert!(error.to_string().contains("requires approval"));
    assert!(!fs::read_to_string(&output_path)
        .unwrap_or_default()
        .contains("Bring an umbrella today"));

    let restarted_audit = ToolAuditStore::new(LocalFolderStorageBackend::new(
        output_path.with_extension("audit"),
    ));
    let denied_lease = restarted_audit
        .fetch_audit("8a7b0000000000000000000000000001_request_weather_lease")
        .expect("durable audit should be readable after vault denial")
        .expect("denied vault lease row should be persisted");
    assert_eq!(denied_lease.tool_id, "vault.request_lease");
    assert_eq!(denied_lease.approval_state, ApprovalState::Pending);
    assert!(!denied_lease.result_summary.ok);
}

#[test]
fn tier2_write_rejects_weak_consent_and_persists_denial() {
    let root = temp_root("weather-agent-tier2-weak-approval-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");
    let config = UmbrellaAgentConfig::deterministic_seattle(&output_path)
        .with_tier2_write_approval(ApprovalAssurance::ExplicitConsent);

    let error = run_umbrella_today_agent(config)
        .expect_err("Tier 2 write should reject weak explicit consent");
    assert!(error.to_string().contains("weaker than required"));
    assert!(!fs::read_to_string(&output_path)
        .unwrap_or_default()
        .contains("Bring an umbrella today"));

    let restarted_audit = ToolAuditStore::new(LocalFolderStorageBackend::new(
        output_path.with_extension("audit"),
    ));
    let denied_write = restarted_audit
        .fetch_audit("8a7b0000000000000000000000000001_write_umbrella_report")
        .expect("durable audit should be readable after weak approval denial")
        .expect("denied Tier 2 write row should be persisted");
    assert_eq!(denied_write.approval_state, ApprovalState::Denied);
    assert!(!denied_write.result_summary.ok);
}

#[test]
fn tier2_write_accepts_challenge_bound_biometric_approval() {
    let root = temp_root("weather-agent-tier2-biometric-approval-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");
    let config = UmbrellaAgentConfig::deterministic_seattle(&output_path)
        .with_tier2_write_approval(ApprovalAssurance::Biometric);

    let run = run_umbrella_today_agent(config)
        .expect("challenge-bound biometric approval should complete the Tier 2 job");
    assert_eq!(run.user_report.write_required_tier, PrivilegeTier::Tier2);
    assert_eq!(
        run.user_report.approval_assurance,
        Some(ApprovalAssurance::Biometric)
    );
    assert_eq!(run.tool_journal_health.approval_biometric_count, 2);
    assert_eq!(run.tool_journal_health.approval_explicit_consent_count, 0);
    assert_eq!(run.durable_audit.approval_granted_records, 2);
    assert!(run.durable_audit.is_complete());
    assert!(run.output_text.contains("Bring an umbrella today"));
}

#[test]
fn successive_ticks_append_to_one_durable_audit_root() {
    let root = temp_root("weather-agent-durable-audit-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let audit_root = root.join("audit");

    let first = run_umbrella_today_agent(
        UmbrellaAgentConfig::deterministic_seattle(root.join("first.txt"))
            .with_audit_root(&audit_root),
    )
    .expect("first tick should complete");
    let second = run_umbrella_today_agent(
        UmbrellaAgentConfig::deterministic_seattle(root.join("second.txt"))
            .with_tick_id("8a7b0000000000000000000000000002")
            .with_audit_root(&audit_root),
    )
    .expect("second tick should append");

    assert!(first.durable_audit.is_complete());
    assert!(second.durable_audit.is_complete());
    let restarted_audit = ToolAuditStore::new(LocalFolderStorageBackend::new(&audit_root));
    assert_eq!(
        restarted_audit
            .query_audits(&ToolAuditRecordQuery::new())
            .expect("both ticks should survive restart")
            .len(),
        8
    );
    assert!(restarted_audit
        .fetch_audit("8a7b0000000000000000000000000002_write_umbrella_report")
        .expect("second write lookup should succeed")
        .is_some());
}

#[test]
fn supervisor_recreates_killed_fetcher_before_pipeline_tick() {
    let root = temp_root("weather-agent-supervisor-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");
    let config = UmbrellaAgentConfig::deterministic_seattle(&output_path)
        .with_killed_child_before_tick("weather-fetcher");

    let run = run_umbrella_today_agent(config).expect("supervised restart should complete");

    assert!(run.output_text.contains("Bring an umbrella today"));
    assert_eq!(run.supervisor.child_count, 3);
    assert_eq!(run.supervisor.stopped_children, 3);
    assert_eq!(run.supervisor.failed_children, 0);
    assert_eq!(run.supervisor.dead_letters, 0);
    assert_eq!(run.supervisor.messages_processed, 3);
    assert_eq!(run.supervisor.killed_children, vec!["weather-fetcher"]);
    assert_eq!(run.supervisor.restarted_children, vec!["weather-fetcher"]);
    assert_eq!(run.supervisor.restart_count, 1);
}

#[test]
#[ignore = "performs live HTTPS requests to api.weather.gov"]
fn umbrella_today_agent_fetches_live_weather_over_tls() {
    let root = temp_root("weather-agent-live-e2e");
    fs::create_dir_all(&root).expect("temp dir should be created");
    let output_path = root.join("umbrella-today.txt");
    let config = UmbrellaAgentConfig::live_seattle_with_timestamp(
        &output_path,
        1_778_624_400_000,
        "2026-05-12T12:00:00.000Z",
    );

    let run = run_umbrella_today_agent(config).expect("live umbrella E2E should complete");

    assert_eq!(
        run.weather_fetch.source_kind,
        WeatherFetchSourceKind::LiveHttps
    );
    assert_eq!(run.weather_fetch.https_requests, 2);
    assert_eq!(run.weather_fetch.tls_handshakes, 2);
    assert_eq!(run.weather_fetch.http_statuses, vec![200, 200]);
    assert_eq!(run.weather_fetch.endpoint_count, 2);
    assert!(run.weather_fetch.http_domain_policy_enforced);
    assert_eq!(
        run.weather_fetch.declared_http_domains,
        vec!["api.weather.gov".to_string()]
    );
    assert!(run
        .weather_fetch
        .forecast_endpoint
        .starts_with("https://api.weather.gov/"));
    assert!(run.output_text.contains("location=Seattle"));
    assert!(run.output_text.contains("decision="));
    if current_kernel_sandbox_support().available {
        assert!(run.kernel_sandbox.network_outbound_enforced);
        assert!(run.kernel_sandbox.network_denied_outbound_blocked);
        assert!(run.kernel_sandbox.weather_https_allowed);
        assert!(!run.kernel_sandbox.host_exact_kernel_enforced);
    }
    assert!(output_path.exists());
}

fn temp_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos()
    ))
}
