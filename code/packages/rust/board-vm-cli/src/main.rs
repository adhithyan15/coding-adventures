use std::env;

use board_vm_cli::{
    format_onboard_led, list_ports, list_targets, parse_args, run_eject_blink, run_esp_detect,
    run_esp_upload, run_repl, run_smoke, usage, CliCommand,
};

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
        CliCommand::ListTargets => {
            for target in list_targets() {
                println!(
                    "{}\t{}\truntime={}\trust_target={}\tled={}\tpins={}\tcaps={}",
                    target.board_id,
                    target.display_name,
                    target.runtime_id,
                    target.rust_target,
                    format_onboard_led(target.onboard_led),
                    target.digital_pin_count,
                    target.capabilities.join(",")
                );
            }
            Ok(())
        }
        CliCommand::EspDetect(options) => {
            let detection = run_esp_detect(&options)?;
            println!(
                "esp chip={} isa={} rust_target={} chip_id={} magic={} api_version={}",
                detection.chip.name(),
                detection.instruction_set.name(),
                detection.rust_target_hint,
                format_optional_u32(detection.chip_id),
                format_optional_hex(detection.magic_value),
                format_optional_u32(detection.api_version)
            );
            Ok(())
        }
        CliCommand::EspUpload(options) => {
            let report = run_esp_upload(&options)?;
            println!(
                "esp-upload image={} offset=0x{:08X} bytes={} block_size={} blocks={} written={} md5={}",
                report.image,
                report.offset,
                report.image_size,
                report.block_size,
                report.block_count,
                report.written_size,
                format_optional_md5(report.md5_digest)
            );
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
        CliCommand::EjectBlink(options) => {
            let report = run_eject_blink(&options)?;
            println!(
                "eject output={} program_id={} slot={} boot_policy={} bytes={} crc32=0x{:08X}",
                report.output,
                report.program_id,
                report.slot,
                report.boot_policy,
                report.module_len,
                report.module_crc32
            );
            Ok(())
        }
        CliCommand::Help => {
            println!("{}", usage());
            Ok(())
        }
    }
}

fn format_optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".to_owned())
}

fn format_optional_hex(value: Option<u32>) -> String {
    value
        .map(|value| format!("0x{value:08X}"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn format_optional_md5(value: Option<[u8; 16]>) -> String {
    value
        .map(|digest| {
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .unwrap_or_else(|| "skipped".to_owned())
}
