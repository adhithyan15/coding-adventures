use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use os_job_core::BackendKind;
use weather_agent_e2e::{run_umbrella_today_agent, RecommendationKind, UmbrellaAgentConfig};

#[test]
fn umbrella_today_agent_exercises_architecture_and_writes_text_file() {
    let root = std::env::temp_dir().join(format!(
        "weather-agent-e2e-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos()
    ));
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

    assert_eq!(run.supervisor.child_count, 3);
    assert_eq!(run.supervisor.stopped_children, 3);
    assert_eq!(run.supervisor.failed_children, 0);
    assert_eq!(run.supervisor.dead_letters, 0);
    assert_eq!(run.supervisor.messages_processed, 3);
    assert_eq!(run.actor_channel_messages, 3);

    assert_eq!(run.tool_journal_health.invocation_count, 3);
    assert_eq!(run.tool_journal_health.completed_count, 3);
    assert_eq!(run.tool_journal_health.failed_count, 0);
    assert!(run.tool_journal_health.results_with_output_count >= 3);
    assert!(run.tool_journal_health.results_with_artifact_refs_count >= 1);

    assert_eq!(run.context_inventory.sessions.total_sessions, 1);
    assert_eq!(run.context_inventory.transcripts.user_entries, 1);
    assert_eq!(run.context_inventory.transcripts.tool_call_entries, 3);
    assert_eq!(run.context_inventory.transcripts.tool_result_entries, 3);
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

    assert_eq!(run.rws.fetcher.untrusted_inputs, 1);
    assert_eq!(run.rws.writer.external_actuations, 1);
    assert!(run.rws.combined_manifest_rejected);
}
