//! The broker process (D18S S-K7, P2.6d-1). Its body is the library's
//! [`chief_of_staff_agent_broker::process::run_inherited`], which every
//! binary that runs a broker shares.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    chief_of_staff_agent_broker::process::run_inherited()
}
