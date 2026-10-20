//! A test fixture: the per-agent channel broker, built inside this crate so
//! its end-to-end tests can launch it next to the real host (D18S
//! P2.6d-2c). Its body is the production broker's,
//! `chief_of_staff_agent_broker::process::run_inherited`.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    chief_of_staff_agent_broker::process::run_inherited()
}
