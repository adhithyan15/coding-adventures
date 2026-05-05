#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(not(target_arch = "arm"))]
fn main() -> std::process::ExitCode {
    use board_vm_uno_r4_firmware::serial_usb_artifact::{
        parse_artifact_args, resolve_rust_llvm_objcopy, usage, ArtifactInvocation,
        SerialUsbArtifactOptions,
    };
    use std::process::ExitCode;

    let mut defaults = SerialUsbArtifactOptions::default();
    if let Some(objcopy) = resolve_rust_llvm_objcopy() {
        defaults.objcopy = objcopy;
    }

    let invocation = match parse_artifact_args(std::env::args().skip(1), defaults) {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
    };

    let ArtifactInvocation::Run(run) = invocation else {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    };

    let commands = match run.options.command_plan() {
        Ok(commands) => commands,
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
    };

    for command in commands {
        println!("+ {}", command.shell_line());
        if run.print_only {
            continue;
        }

        let status = match command.to_command().status() {
            Ok(status) => status,
            Err(error) => {
                eprintln!("error: failed to start command: {error}");
                return ExitCode::FAILURE;
            }
        };

        if !status.success() {
            eprintln!("error: command exited with {status}");
            return ExitCode::from(status.code().unwrap_or(1) as u8);
        }
    }

    ExitCode::SUCCESS
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
