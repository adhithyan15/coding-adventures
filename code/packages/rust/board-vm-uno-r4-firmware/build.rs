use std::env;
use std::fs;
use std::path::PathBuf;

const UNO_R4_SKETCH_FLASH_ORIGIN: u32 = 0x0000_4000;
const UNO_R4_CODE_FLASH_BYTES: u32 = 0x0004_0000;
const UNO_R4_RAM_ORIGIN: u32 = 0x2000_0000;
const UNO_R4_RAM_BYTES: u32 = 0x8000;
const FIRMWARE_BINS: [&str; 5] = [
    "uno-r4-vm-blink-smoke",
    "uno-r4-wifi-raw-blink-probe",
    "uno-r4-wifi-stream-handshake-probe",
    "uno-r4-wifi-stream-session-probe",
    "uno-r4-wifi-uart-server",
];

fn main() {
    if env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("arm") {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo"));
        let flash_len = UNO_R4_CODE_FLASH_BYTES - UNO_R4_SKETCH_FLASH_ORIGIN;
        fs::write(
            out_dir.join("memory.x"),
            format!(
                "MEMORY\n{{\n  FLASH : ORIGIN = 0x{UNO_R4_SKETCH_FLASH_ORIGIN:08X}, LENGTH = 0x{flash_len:X}\n  RAM : ORIGIN = 0x{UNO_R4_RAM_ORIGIN:08X}, LENGTH = 0x{UNO_R4_RAM_BYTES:X}\n}}\n\n_stack_start = ORIGIN(RAM) + LENGTH(RAM);\n"
            ),
        )
        .expect("write Uno R4 memory.x");

        println!("cargo:rustc-link-search={}", out_dir.display());
        for firmware_bin in FIRMWARE_BINS {
            println!("cargo:rustc-link-arg-bin={firmware_bin}=-Tlink.x");
        }
    }
}
