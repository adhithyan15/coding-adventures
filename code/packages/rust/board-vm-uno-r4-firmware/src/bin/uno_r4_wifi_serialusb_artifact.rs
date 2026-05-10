#![cfg_attr(target_arch = "arm", no_std)]
#![cfg_attr(target_arch = "arm", no_main)]

#[cfg(not(target_arch = "arm"))]
use board_vm_uno_r4_firmware::serial_usb_artifact::CommandSpec;
#[cfg(not(target_arch = "arm"))]
use std::io::{self, Write};
#[cfg(not(target_arch = "arm"))]
use std::process::{ExitCode, ExitStatus, Output};

#[cfg(not(target_arch = "arm"))]
fn main() -> std::process::ExitCode {
    use board_vm_uno_r4_firmware::serial_usb_artifact::{
        parse_artifact_args, resolve_rust_llvm_objcopy, usage, ArtifactInvocation,
        SerialUsbArtifactOptions,
    };

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

    let ArtifactInvocation::Run(mut run) = invocation else {
        println!("{}", usage());
        return ExitCode::SUCCESS;
    };
    run.options.fill_host_defaults();

    if run.print_only {
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
        }

        return ExitCode::SUCCESS;
    }

    if let Err(code) = run_command(&run.options.build_command()) {
        return code;
    }
    if let Err(code) = run_command(&run.options.objcopy_command()) {
        return code;
    }

    let mut upload_output_text = String::new();
    if run.options.upload {
        let requested_port = match run.options.upload_port() {
            Ok(port) => port,
            Err(error) => {
                eprintln!("error: {error}");
                eprintln!("{}", usage());
                return ExitCode::from(2);
            }
        };
        let upload_port = if run.options.touch_bootloader {
            println!("+ touch-arduino-bootloader --port {requested_port} --baud 1200");
            match run.options.touch_bootloader_port(requested_port) {
                Ok(port) => {
                    println!("+ bootloader upload port {port}");
                    port
                }
                Err(error) => {
                    eprintln!("error: {error}");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            requested_port.to_string()
        };

        let command = run.options.upload_command_for_port(&upload_port);

        if run.options.smoke {
            let output = match run_command_capture(&command) {
                Ok(output) => output,
                Err(code) => return code,
            };
            upload_output_text = output_text(&output);
        } else if let Err(code) = run_command(&command) {
            return code;
        }
    }

    if run.options.smoke {
        let port = match run
            .options
            .smoke_port_after_upload_output(&upload_output_text)
        {
            Ok(port) => port,
            Err(error) => {
                eprintln!("error: {error}");
                eprintln!("{}", usage());
                return ExitCode::from(2);
            }
        };
        if let Err(code) = run_command(&run.options.smoke_command_for_port(&port)) {
            return code;
        }
    }

    ExitCode::SUCCESS
}

#[cfg(not(target_arch = "arm"))]
fn run_command(command: &CommandSpec) -> Result<(), ExitCode> {
    println!("+ {}", command.shell_line());
    let status = command_status(command)?;
    if !status.success() {
        eprintln!("error: command exited with {status}");
        return Err(exit_code_from_status(&status));
    }

    Ok(())
}

#[cfg(not(target_arch = "arm"))]
fn run_command_capture(command: &CommandSpec) -> Result<Output, ExitCode> {
    println!("+ {}", command.shell_line());
    let output = match command.to_command().output() {
        Ok(output) => output,
        Err(error) => {
            eprintln!("error: failed to start command: {error}");
            return Err(ExitCode::FAILURE);
        }
    };

    if let Err(error) = io::stdout().write_all(&output.stdout) {
        eprintln!("error: failed to write command stdout: {error}");
        return Err(ExitCode::FAILURE);
    }
    if let Err(error) = io::stderr().write_all(&output.stderr) {
        eprintln!("error: failed to write command stderr: {error}");
        return Err(ExitCode::FAILURE);
    }

    if !output.status.success() {
        eprintln!("error: command exited with {}", output.status);
        return Err(exit_code_from_status(&output.status));
    }

    Ok(output)
}

#[cfg(not(target_arch = "arm"))]
fn command_status(command: &CommandSpec) -> Result<ExitStatus, ExitCode> {
    command.to_command().status().map_err(|error| {
        eprintln!("error: failed to start command: {error}");
        ExitCode::FAILURE
    })
}

#[cfg(not(target_arch = "arm"))]
fn output_text(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if !text.ends_with('\n') && !output.stderr.is_empty() {
        text.push('\n');
    }
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

#[cfg(not(target_arch = "arm"))]
fn exit_code_from_status(status: &ExitStatus) -> ExitCode {
    let code = status.code().unwrap_or(1);
    if (0..=255).contains(&code) {
        ExitCode::from(code as u8)
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(target_arch = "arm")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {}
}
