//! Process entry point for the D18 Chief daemon.
//!
//! The first thing it does, before it reads a config, a key or the vault,
//! is suppress its own core dumps and debugger attach (D18S S-I5): the
//! daemon is about to hold every agent's channel keys. If that fails, it
//! refuses to start rather than run exposed (S-P3).

fn main() {
    match chief_of_staff_process_hardening::suppress_core_dumps() {
        Ok(protection) => {
            for missing in protection.missing {
                eprintln!("chief daemon: not yet hardened on this platform: {missing}");
            }
        }
        Err(error) => {
            eprintln!("chief daemon: refusing to start: {error}");
            std::process::exit(1);
        }
    }
    if let Err(error) = chief_of_staff_daemon::run_from_env() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
