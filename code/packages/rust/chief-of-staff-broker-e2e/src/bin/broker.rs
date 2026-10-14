//! The broker, built inside this test crate so its tests can launch it next
//! to the test host. Its body is the production broker's:
//! `chief_of_staff_agent_broker::process::run_inherited`.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    chief_of_staff_agent_broker::process::run_inherited()
}
