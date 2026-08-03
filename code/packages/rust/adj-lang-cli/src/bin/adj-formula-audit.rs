//! Emit a canonical, replayable audit of every formula query in an ADJ program.

fn main() -> std::process::ExitCode {
    adj_lang_cli::formula_audit::main_entry()
}
