use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[allow(dead_code)]
mod arduino_usb_link {
    include!("src/arduino_usb_link.rs");
}

use arduino_usb_link::{
    ArduinoSourceLanguage, ARDUINO_ARM_AR_ENV_VAR, ARDUINO_ARM_GCC_ENV_VAR,
    ARDUINO_ARM_GXX_ENV_VAR, ARDUINO_CORE_ENV_VAR, ARDUINO_USB_LINK_CFLAGS,
    ARDUINO_USB_LINK_CXXFLAGS, ARDUINO_USB_LINK_DEFINES, ARDUINO_USB_LINK_ENV_VAR,
    ARDUINO_USB_LINK_INCLUDE_DIRS, ARDUINO_USB_LINK_SOURCES, UNO_R4_WIFI_FSP_ARCHIVE,
};

const UNO_R4_SKETCH_FLASH_ORIGIN: u32 = 0x0000_4000;
const UNO_R4_CODE_FLASH_BYTES: u32 = 0x0004_0000;
const UNO_R4_RAM_ORIGIN: u32 = 0x2000_0000;
const UNO_R4_RAM_BYTES: u32 = 0x8000;
const ARDUINO_ARM_GCC_VERSION: &str = "7-2017q4";
const ARDUINO_USB_ARCHIVE: &str = "board_vm_uno_r4_arduino_usb";
const FIRMWARE_BINS: [&str; 6] = [
    "uno-r4-vm-blink-smoke",
    "uno-r4-wifi-raw-blink-probe",
    "uno-r4-wifi-stream-handshake-probe",
    "uno-r4-wifi-stream-session-probe",
    "uno-r4-wifi-uart-server",
    "uno-r4-wifi-serialusb-server",
];

fn main() {
    emit_rerun_hints();

    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("arm") {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
        write_memory_x(&out_dir);

        println!("cargo:rustc-link-search={}", out_dir.display());
        for firmware_bin in FIRMWARE_BINS {
            println!("cargo:rustc-link-arg-bin={firmware_bin}=-Tlink.x");
        }

        if env_enabled(ARDUINO_USB_LINK_ENV_VAR) {
            compile_and_link_arduino_usb(&out_dir);
        }
    }
}

fn emit_rerun_hints() {
    println!("cargo:rerun-if-env-changed={ARDUINO_USB_LINK_ENV_VAR}");
    println!("cargo:rerun-if-env-changed={ARDUINO_CORE_ENV_VAR}");
    println!("cargo:rerun-if-env-changed={ARDUINO_ARM_GCC_ENV_VAR}");
    println!("cargo:rerun-if-env-changed={ARDUINO_ARM_GXX_ENV_VAR}");
    println!("cargo:rerun-if-env-changed={ARDUINO_ARM_AR_ENV_VAR}");
}

fn write_memory_x(out_dir: &Path) {
    let flash_len = UNO_R4_CODE_FLASH_BYTES - UNO_R4_SKETCH_FLASH_ORIGIN;
    fs::write(
        out_dir.join("memory.x"),
        format!(
            "MEMORY\n{{\n  FLASH : ORIGIN = 0x{UNO_R4_SKETCH_FLASH_ORIGIN:08X}, LENGTH = 0x{flash_len:X}\n  RAM : ORIGIN = 0x{UNO_R4_RAM_ORIGIN:08X}, LENGTH = 0x{UNO_R4_RAM_BYTES:X}\n}}\n\n_stack_start = ORIGIN(RAM) + LENGTH(RAM);\n"
        ),
    )
    .expect("write Uno R4 memory.x");
}

fn compile_and_link_arduino_usb(out_dir: &Path) {
    let core_dir = resolve_arduino_core_dir();
    let packages_dir = arduino_packages_dir(&core_dir);
    let gcc = resolve_tool(ARDUINO_ARM_GCC_ENV_VAR, &packages_dir, "gcc");
    let gxx = resolve_tool(ARDUINO_ARM_GXX_ENV_VAR, &packages_dir, "g++");
    let ar = resolve_tool(ARDUINO_ARM_AR_ENV_VAR, &packages_dir, "ar");

    let object_dir = out_dir.join("arduino-usb-objects");
    fs::create_dir_all(&object_dir).expect("create Arduino USB object dir");

    let mut objects = Vec::new();
    for source in ARDUINO_USB_LINK_SOURCES {
        let source_path = core_dir.join(source.path);
        println!("cargo:rerun-if-changed={}", source_path.display());

        match source.language {
            ArduinoSourceLanguage::C => {
                let object = object_dir.join(object_name(source.path));
                compile_source(
                    &gcc,
                    &core_dir,
                    &source_path,
                    &object,
                    ARDUINO_USB_LINK_CFLAGS,
                );
                objects.push(object);
            }
            ArduinoSourceLanguage::Cxx => {
                let object = object_dir.join(object_name(source.path));
                compile_source(
                    &gxx,
                    &core_dir,
                    &source_path,
                    &object,
                    ARDUINO_USB_LINK_CXXFLAGS,
                );
                objects.push(object);
            }
            ArduinoSourceLanguage::StaticArchive => {
                let archive = core_dir.join(source.path);
                assert_path_exists(&archive, "Arduino static archive");
                println!("cargo:rerun-if-changed={}", archive.display());
            }
        }
    }

    let archive_path = out_dir.join(format!("lib{ARDUINO_USB_ARCHIVE}.a"));
    if archive_path.exists() {
        fs::remove_file(&archive_path).expect("remove stale Arduino USB archive");
    }
    run_command(
        Command::new(&ar)
            .arg("rcs")
            .arg(&archive_path)
            .args(&objects),
        "archive Arduino USB objects",
    );

    let fsp_archive = core_dir.join(UNO_R4_WIFI_FSP_ARCHIVE);
    assert_path_exists(&fsp_archive, "Uno R4 WiFi FSP archive");
    let fsp_archive_dir = fsp_archive
        .parent()
        .expect("FSP archive lives under a directory");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!(
        "cargo:rustc-link-search=native={}",
        fsp_archive_dir.display()
    );
    emit_serial_usb_link_arg("--whole-archive");
    emit_serial_usb_link_arg("--start-group");
    emit_serial_usb_link_arg(&format!("-l{ARDUINO_USB_ARCHIVE}"));
    emit_serial_usb_link_arg("-lfsp");
    emit_serial_usb_link_arg("--no-whole-archive");
    emit_serial_usb_link_arg("--end-group");
}

fn compile_source(
    compiler: &Path,
    core_dir: &Path,
    source_path: &Path,
    object_path: &Path,
    flags: &[&str],
) {
    assert_path_exists(source_path, "Arduino source");

    let mut command = Command::new(compiler);
    command.args(flags);
    for define in ARDUINO_USB_LINK_DEFINES {
        command.arg(format!("-D{define}"));
    }
    for include in ARDUINO_USB_LINK_INCLUDE_DIRS {
        command.arg(format!("-I{}", core_dir.join(include).display()));
    }
    command.arg("-o").arg(object_path).arg(source_path);

    run_command(&mut command, "compile Arduino USB source");
}

fn env_enabled(name: &str) -> bool {
    env::var(name)
        .map(|value| {
            matches!(
                value.as_str(),
                "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
            )
        })
        .unwrap_or(false)
}

fn resolve_arduino_core_dir() -> PathBuf {
    if let Some(path) = env::var_os(ARDUINO_CORE_ENV_VAR) {
        let path = PathBuf::from(path);
        assert_path_exists(&path, "Arduino Renesas UNO core");
        return path;
    }

    let home = PathBuf::from(env::var_os("HOME").unwrap_or_default());
    let candidates = [
        home.join("Library/Arduino15/packages/arduino/hardware/renesas_uno/1.5.3"),
        home.join(".arduino15/packages/arduino/hardware/renesas_uno/1.5.3"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }

    panic!(
        "{ARDUINO_CORE_ENV_VAR} must point to the Arduino Renesas UNO core when {ARDUINO_USB_LINK_ENV_VAR}=1"
    );
}

fn arduino_packages_dir(core_dir: &Path) -> PathBuf {
    core_dir
        .ancestors()
        .nth(4)
        .expect("Arduino core path is under packages/arduino/hardware")
        .to_path_buf()
}

fn resolve_tool(env_var: &str, packages_dir: &Path, suffix: &str) -> PathBuf {
    if let Some(path) = env::var_os(env_var) {
        let path = PathBuf::from(path);
        assert_path_exists(&path, "Arduino ARM tool");
        return path;
    }

    let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
    let tool = packages_dir
        .join("arduino/tools/arm-none-eabi-gcc")
        .join(ARDUINO_ARM_GCC_VERSION)
        .join("bin")
        .join(format!("arm-none-eabi-{suffix}{exe_suffix}"));
    assert_path_exists(&tool, "Arduino ARM tool");
    tool
}

fn assert_path_exists(path: &Path, label: &str) {
    assert!(path.exists(), "{label} does not exist: {}", path.display());
}

fn object_name(source_path: &str) -> String {
    let mut name = source_path.replace(['/', '\\', '.'], "_");
    name.push_str(".o");
    name
}

fn emit_serial_usb_link_arg(arg: &str) {
    println!("cargo:rustc-link-arg-bin=uno-r4-wifi-serialusb-server={arg}");
}

fn run_command(command: &mut Command, action: &str) {
    let status = command.status().unwrap_or_else(|error| {
        panic!(
            "{action} failed to start: {error}. If the Arduino-packaged ARM toolchain cannot run on this host, set {ARDUINO_ARM_GCC_ENV_VAR}, {ARDUINO_ARM_GXX_ENV_VAR}, and {ARDUINO_ARM_AR_ENV_VAR} to compatible arm-none-eabi tools."
        );
    });
    assert!(status.success(), "{action} failed with status {status}");
}
