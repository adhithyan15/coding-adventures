use std::env;

use board_vm_cli::{list_ports, parse_args, run_repl, run_smoke, usage, CliCommand};

fn main() {
    match parse_args(env::args().skip(1)).and_then(run_command) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("error: {error}\n\n{}", usage());
            std::process::exit(2);
        }
    }
}

fn run_command(command: CliCommand) -> Result<(), board_vm_cli::CliError> {
    match command {
        CliCommand::ListPorts => {
            for port in list_ports()? {
                println!("{}\t{:?}", port.port_name, port.port_type);
            }
            Ok(())
        }
        CliCommand::Smoke(options) => {
            let report = run_smoke(&options)?;
            println!(
                "hello board={} runtime={} protocol={} host_nonce=0x{:08X} board_nonce=0x{:08X}",
                report.hello.board_name,
                report.hello.runtime_name,
                report.hello.selected_version,
                report.hello.host_nonce,
                report.hello.board_nonce
            );
            println!(
                "caps board={} runtime={} max_program_bytes={} stack={} handles={} capabilities={}",
                report.descriptor.board_id,
                report.descriptor.runtime_id,
                report.descriptor.max_program_bytes,
                report.descriptor.max_stack_values,
                report.descriptor.max_handles,
                report.descriptor.capabilities.len()
            );
            println!(
                "blink program_id={} status={:?} instructions={} elapsed_ms={} open_handles={}",
                report.run.program_id,
                report.run.status,
                report.run.instructions_executed,
                report.run.elapsed_ms,
                report.run.open_handles
            );
            Ok(())
        }
        CliCommand::Repl(options) => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            run_repl(&options, stdin.lock(), stdout.lock())
        }
        CliCommand::Help => {
            println!("{}", usage());
            Ok(())
        }
    }
}
