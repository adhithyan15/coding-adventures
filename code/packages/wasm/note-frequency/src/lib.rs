use note_frequency::{note_to_frequency, parse_note, Note};
use wasm_bindgen::prelude::*;

fn to_js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}

#[wasm_bindgen]
pub struct WasmNote {
    inner: Note,
}

#[wasm_bindgen]
impl WasmNote {
    #[wasm_bindgen(getter)]
    pub fn letter(&self) -> String {
        self.inner.letter.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn accidental(&self) -> String {
        self.inner.accidental.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn octave(&self) -> i32 {
        self.inner.octave
    }

    #[wasm_bindgen(js_name = "spelling")]
    pub fn spelling(&self) -> String {
        self.inner.spelling()
    }

    #[wasm_bindgen(js_name = "semitonesFromA4")]
    pub fn semitones_from_a4(&self) -> i32 {
        self.inner.semitones_from_a4()
    }

    pub fn frequency(&self) -> f64 {
        self.inner.frequency()
    }

    #[wasm_bindgen(js_name = "toString")]
    pub fn to_string_value(&self) -> String {
        self.inner.to_string()
    }
}

#[wasm_bindgen(js_name = "parseNote")]
pub fn parse_note_js(text: &str) -> Result<WasmNote, JsValue> {
    Ok(WasmNote {
        inner: parse_note(text).map_err(to_js_error)?,
    })
}

#[wasm_bindgen(js_name = "noteToFrequency")]
pub fn note_to_frequency_js(text: &str) -> Result<f64, JsValue> {
    note_to_frequency(text).map_err(to_js_error)
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrapper_preserves_fields() {
        let note = parse_note_js("C#5").unwrap();
        assert_eq!(note.letter(), "C");
        assert_eq!(note.accidental(), "#");
        assert_eq!(note.octave(), 5);
    }

    #[test]
    fn wrapper_maps_frequencies() {
        assert!((note_to_frequency_js("C4").unwrap() - 261.6255653005986).abs() < 1e-12);
    }
}
