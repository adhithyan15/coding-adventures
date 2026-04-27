#![allow(non_camel_case_types, non_snake_case)]

use std::ffi::c_void;
use std::ptr;

use coding_adventures_json_value::{parse, JsonNumber, JsonValue};
use node_bridge::{
    create_function, get_cb_info, napi_callback_info, napi_env, napi_status, napi_value,
    set_named_property, str_from_js, str_to_js, throw_error, undefined, NAPI_OK,
};
use paint_codec_png::encode_png;
use paint_instructions::{PaintBase, PaintInstruction, PaintRect, PaintScene};

extern "C" {
    fn napi_create_buffer_copy(
        env: napi_env,
        length: usize,
        data: *const c_void,
        result_data: *mut *mut c_void,
        result: *mut napi_value,
    ) -> napi_status;
}

fn json_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Object(_) => "object",
        JsonValue::Array(_) => "array",
        JsonValue::String(_) => "string",
        JsonValue::Number(_) => "number",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Null => "null",
    }
}

fn expect_object<'a>(value: &'a JsonValue, path: &str) -> Result<&'a Vec<(String, JsonValue)>, String> {
    match value {
        JsonValue::Object(entries) => Ok(entries),
        _ => Err(format!("{path} must be an object, got {}", json_type_name(value))),
    }
}

fn expect_array<'a>(value: &'a JsonValue, path: &str) -> Result<&'a Vec<JsonValue>, String> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err(format!("{path} must be an array, got {}", json_type_name(value))),
    }
}

fn expect_string(value: &JsonValue, path: &str) -> Result<String, String> {
    match value {
        JsonValue::String(text) => Ok(text.clone()),
        _ => Err(format!("{path} must be a string, got {}", json_type_name(value))),
    }
}

fn expect_f64(value: &JsonValue, path: &str) -> Result<f64, String> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(*value as f64),
        JsonValue::Number(JsonNumber::Float(value)) => Ok(*value),
        _ => Err(format!("{path} must be a number, got {}", json_type_name(value))),
    }
}

fn required_field<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
    path: &str,
) -> Result<&'a JsonValue, String> {
    object
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
        .ok_or_else(|| format!("{path}.{name} is required"))
}

fn optional_string_field(
    object: &[(String, JsonValue)],
    name: &str,
    path: &str,
) -> Result<Option<String>, String> {
    object
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| expect_string(value, &format!("{path}.{name}")))
        .transpose()
}

fn optional_f64_field(
    object: &[(String, JsonValue)],
    name: &str,
    path: &str,
) -> Result<Option<f64>, String> {
    object
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| expect_f64(value, &format!("{path}.{name}")))
        .transpose()
}

fn parse_rect(value: &JsonValue, path: &str) -> Result<PaintInstruction, String> {
    let object = expect_object(value, path)?;

    Ok(PaintInstruction::Rect(PaintRect {
        base: PaintBase {
            id: optional_string_field(object, "id", path)?,
            metadata: None,
        },
        x: expect_f64(required_field(object, "x", path)?, &format!("{path}.x"))?,
        y: expect_f64(required_field(object, "y", path)?, &format!("{path}.y"))?,
        width: expect_f64(required_field(object, "width", path)?, &format!("{path}.width"))?,
        height: expect_f64(required_field(object, "height", path)?, &format!("{path}.height"))?,
        fill: optional_string_field(object, "fill", path)?,
        stroke: optional_string_field(object, "stroke", path)?,
        stroke_width: optional_f64_field(object, "stroke_width", path)?,
        corner_radius: optional_f64_field(object, "corner_radius", path)?,
        stroke_dash: None,
        stroke_dash_offset: None,
    }))
}

fn parse_instruction(value: &JsonValue, path: &str) -> Result<PaintInstruction, String> {
    let object = expect_object(value, path)?;
    let kind = expect_string(required_field(object, "kind", path)?, &format!("{path}.kind"))?;

    match kind.as_str() {
        "rect" => parse_rect(value, path),
        unsupported => Err(format!(
            "{path}.kind {:?} is not supported by the native barcode renderer yet",
            unsupported
        )),
    }
}

fn parse_scene(scene_json: &str) -> Result<PaintScene, String> {
    let scene_value = parse(scene_json).map_err(|error| error.to_string())?;
    let scene_object = expect_object(&scene_value, "scene")?;
    let instructions_value = required_field(scene_object, "instructions", "scene")?;
    let instructions_array = expect_array(instructions_value, "scene.instructions")?;

    let mut instructions = Vec::with_capacity(instructions_array.len());
    for (index, instruction_value) in instructions_array.iter().enumerate() {
        instructions.push(parse_instruction(
            instruction_value,
            &format!("scene.instructions[{index}]"),
        )?);
    }

    Ok(PaintScene {
        width: expect_f64(required_field(scene_object, "width", "scene")?, "scene.width")?,
        height: expect_f64(required_field(scene_object, "height", "scene")?, "scene.height")?,
        background: expect_string(
            required_field(scene_object, "background", "scene")?,
            "scene.background",
        )?,
        instructions,
        id: optional_string_field(scene_object, "id", "scene")?,
        metadata: None,
    })
}

#[cfg(target_os = "macos")]
fn render_pixels(scene: &PaintScene) -> Result<paint_instructions::PixelContainer, String> {
    Ok(paint_metal::render(scene))
}

#[cfg(target_os = "windows")]
fn render_pixels(scene: &PaintScene) -> Result<paint_instructions::PixelContainer, String> {
    match std::panic::catch_unwind(|| paint_vm_direct2d::render(scene)) {
        Ok(pixels) => Ok(pixels),
        Err(_) => Ok(paint_vm_gdi::render(scene)),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn render_pixels(_scene: &PaintScene) -> Result<paint_instructions::PixelContainer, String> {
    Err(
        "barcode-1d native rendering currently supports only macOS (paint-metal) and Windows (Direct2D/GDI)"
            .to_string(),
    )
}

#[cfg(target_os = "macos")]
fn paint_backend_name() -> &'static str {
    "paint-metal"
}

#[cfg(target_os = "windows")]
fn paint_backend_name() -> &'static str {
    "paint-vm-direct2d"
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn paint_backend_name() -> &'static str {
    "unsupported"
}

unsafe fn bytes_to_buffer(env: napi_env, bytes: &[u8]) -> napi_value {
    let mut result: napi_value = ptr::null_mut();
    let status = napi_create_buffer_copy(
        env,
        bytes.len(),
        bytes.as_ptr() as *const c_void,
        ptr::null_mut(),
        &mut result,
    );

    if status != NAPI_OK {
        throw_error(env, "failed to create Buffer for PNG bytes");
        return undefined(env);
    }

    result
}

unsafe extern "C" fn node_render_scene_to_png(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);

    if args.is_empty() {
        throw_error(env, "renderSceneToPng requires one string argument containing a PaintScene JSON payload");
        return undefined(env);
    }

    let scene_json = match str_from_js(env, args[0]) {
        Some(scene_json) => scene_json,
        None => {
            throw_error(env, "renderSceneToPng requires a JSON string");
            return undefined(env);
        }
    };

    let scene = match parse_scene(&scene_json) {
        Ok(scene) => scene,
        Err(error) => {
            throw_error(env, &error);
            return undefined(env);
        }
    };

    let pixels = match render_pixels(&scene) {
        Ok(pixels) => pixels,
        Err(error) => {
            throw_error(env, &error);
            return undefined(env);
        }
    };

    let png = encode_png(&pixels);
    bytes_to_buffer(env, &png)
}

unsafe extern "C" fn node_get_paint_backend(
    env: napi_env,
    _info: napi_callback_info,
) -> napi_value {
    str_to_js(env, paint_backend_name())
}

#[no_mangle]
pub unsafe extern "C" fn napi_register_module_v1(
    env: napi_env,
    exports: napi_value,
) -> napi_value {
    let render_scene_to_png_fn =
        create_function(env, "renderSceneToPng", Some(node_render_scene_to_png));
    set_named_property(env, exports, "renderSceneToPng", render_scene_to_png_fn);

    let get_paint_backend_fn =
        create_function(env, "getPaintBackend", Some(node_get_paint_backend));
    set_named_property(env, exports, "getPaintBackend", get_paint_backend_fn);

    exports
}
