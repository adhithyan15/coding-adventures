use std::{env, error::Error, fs, io};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let input = args.next().ok_or("expected WAT input path")?;
    let output = args.next().ok_or("expected Wasm output path")?;
    if args.next().is_some() {
        return Err("expected exactly an input and output path".into());
    }

    let source = fs::read_to_string(input)?;
    let module = wasm_wast_parser::parse_module(&source)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let bytes = wasm_module_encoder::encode_module(&module)?;
    fs::write(output, bytes)?;
    Ok(())
}
