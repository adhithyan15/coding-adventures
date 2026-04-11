//! JSON-RPC 2.0 message types: `Request`, `Response`, `Notification`, and the
//! `Message` enum that is their discriminated union.
//!
//! ## The Four Message Types
//!
//! All messages carry `"jsonrpc": "2.0"`. The type is determined by the fields
//! present:
//!
//! | Fields present              | Type         |
//! |-----------------------------|--------------|
//! | `id` + `method`             | Request      |
//! | `method` only (no `id`)     | Notification |
//! | `result` or `error`         | Response     |
//!
//! ### Request
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"method":"textDocument/hover","params":{...}}
//! ```
//!
//! ### Response (success)
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"result":{"contents":"**INC**"}}
//! ```
//!
//! ### Response (error)
//!
//! ```json
//! {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
//! ```
//!
//! ### Notification
//!
//! ```json
//! {"jsonrpc":"2.0","method":"textDocument/didOpen","params":{...}}
//! ```
//!
//! ## Parsing Strategy
//!
//! `parse_message` deserialises the JSON into a `serde_json::Value`, then
//! inspects the top-level keys to determine which struct to build. We do NOT
//! use `#[serde(untagged)]` on the enum because the disambiguation logic (the
//! presence/absence of `id`) is not expressible with serde's built-in tags.

use crate::errors::ResponseError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// The id type
// ---------------------------------------------------------------------------
//
// A JSON-RPC id is either a string or an integer. We use serde_json::Value
// to represent it so we don't have to define a custom enum. We only accept
// Value::String, Value::Number, and Value::Null (the null case is only valid
// in error responses where the request id was undeterminable).

/// A JSON-RPC message id — string, integer, or null.
///
/// Use `Value::String("abc".into())` for string ids, `Value::Number(1.into())`
/// for integer ids, and `Value::Null` for null (error response only).
pub type Id = Value;

// ---------------------------------------------------------------------------
// Request
// ---------------------------------------------------------------------------

/// A JSON-RPC Request. Has `id`, `method`, and optional `params`.
///
/// The server must send a `Response` with the same `id`.
///
/// # Example
///
/// ```json
/// {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"rootUri":null}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// Correlates the response back to this request. Must not be null.
    pub id: Id,
    /// The procedure name, e.g. `"textDocument/hover"`.
    pub method: String,
    /// Optional parameters — object or array.
    pub params: Option<Value>,
}

// ---------------------------------------------------------------------------
// Response
// ---------------------------------------------------------------------------

/// A JSON-RPC Response. Has `id` and either `result` (success) or `error`.
///
/// The `id` must match the originating Request. It may be null only when the
/// server could not determine the request id (e.g. the request was unparseable).
///
/// # Example (success)
///
/// ```json
/// {"jsonrpc":"2.0","id":1,"result":{"capabilities":{}}}
/// ```
///
/// # Example (error)
///
/// ```json
/// {"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    /// The id from the originating Request.
    pub id: Id,
    /// The success result. Mutually exclusive with `error`.
    pub result: Option<Value>,
    /// The error object. Mutually exclusive with `result`.
    pub error: Option<ResponseError>,
}

// ---------------------------------------------------------------------------
// Notification
// ---------------------------------------------------------------------------

/// A JSON-RPC Notification. Has `method` but no `id`.
///
/// The server **must not** send a response to a Notification.
///
/// # Example
///
/// ```json
/// {"jsonrpc":"2.0","method":"textDocument/didChange","params":{...}}
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    /// The event name, e.g. `"textDocument/didOpen"`.
    pub method: String,
    /// Optional parameters.
    pub params: Option<Value>,
}

// ---------------------------------------------------------------------------
// Message — discriminated union
// ---------------------------------------------------------------------------

/// A JSON-RPC message — one of `Request`, `Response`, or `Notification`.
///
/// Use pattern matching to handle each variant:
///
/// ```rust
/// use coding_adventures_json_rpc::message::{Message, Request};
///
/// fn handle(msg: Message) {
///     match msg {
///         Message::Request(req) => println!("request: {}", req.method),
///         Message::Response(resp) => println!("response id: {:?}", resp.id),
///         Message::Notification(notif) => println!("notification: {}", notif.method),
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// A client-to-server call expecting a response.
    Request(Request),
    /// A server-to-client reply (or a response to any request).
    Response(Response),
    /// A one-way message with no response.
    Notification(Notification),
}

// ---------------------------------------------------------------------------
// parse_message — binary JSON → typed Message
// ---------------------------------------------------------------------------

/// Parse a JSON byte slice into a typed `Message`.
///
/// Returns `Ok(Message)` on success, or `Err(ResponseError)` when:
/// - The bytes are not valid JSON → `PARSE_ERROR (-32700)`
/// - The JSON is valid but not a JSON-RPC object → `INVALID_REQUEST (-32600)`
///
/// ## Discrimination Logic
///
/// 1. Decode JSON.
/// 2. Top-level value must be an object.
/// 3. Has `"method"` + `"id"` → `Request`.
/// 4. Has `"method"` but no `"id"` → `Notification`.
/// 5. Has `"result"` or `"error"` → `Response`.
/// 6. None of the above → `INVALID_REQUEST`.
///
/// # Examples
///
/// ```rust
/// use coding_adventures_json_rpc::message::{parse_message, Message};
///
/// let json = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
/// let msg = parse_message(json).unwrap();
/// assert!(matches!(msg, Message::Request(_)));
///
/// let bad = b"not json";
/// assert!(parse_message(bad).is_err());
/// ```
pub fn parse_message(json: &[u8]) -> Result<Message, ResponseError> {
    // Step 1: Parse bytes as JSON.
    let value: Value = serde_json::from_slice(json).map_err(|e| {
        ResponseError::parse_error(Some(Value::String(e.to_string())))
    })?;

    // Step 2: Must be a JSON object.
    let obj = match &value {
        Value::Object(map) => map,
        _ => {
            return Err(ResponseError::invalid_request(Some(Value::String(
                "expected JSON object".to_string(),
            ))))
        }
    };

    let has_method = obj.contains_key("method");
    let has_id = obj.contains_key("id");
    let has_result = obj.contains_key("result");
    let has_error = obj.contains_key("error");

    if has_method && has_id {
        // Request — must have both "method" and "id".
        let id = obj["id"].clone();
        let method = match &obj["method"] {
            Value::String(s) => s.clone(),
            _ => {
                return Err(ResponseError::invalid_request(Some(Value::String(
                    "method must be a string".to_string(),
                ))))
            }
        };
        let params = obj.get("params").cloned();
        Ok(Message::Request(Request { id, method, params }))
    } else if has_method {
        // Notification — "method" present, "id" absent.
        let method = match &obj["method"] {
            Value::String(s) => s.clone(),
            _ => {
                return Err(ResponseError::invalid_request(Some(Value::String(
                    "method must be a string".to_string(),
                ))))
            }
        };
        let params = obj.get("params").cloned();
        Ok(Message::Notification(Notification { method, params }))
    } else if has_result || has_error {
        // Response — "result" or "error" present.
        let id = obj.get("id").cloned().unwrap_or(Value::Null);
        let result = obj.get("result").cloned();
        let error = if let Some(err_val) = obj.get("error") {
            Some(serde_json::from_value(err_val.clone()).map_err(|e| {
                ResponseError::parse_error(Some(Value::String(format!(
                    "invalid error object: {}",
                    e
                ))))
            })?)
        } else {
            None
        };
        Ok(Message::Response(Response { id, result, error }))
    } else {
        Err(ResponseError::invalid_request(Some(Value::String(
            "missing 'method', 'result', or 'error' field".to_string(),
        ))))
    }
}

// ---------------------------------------------------------------------------
// message_to_value — typed Message → serde_json::Value for serialisation
// ---------------------------------------------------------------------------

/// Convert a typed `Message` to a `serde_json::Value` ready for serialisation.
///
/// Always injects `"jsonrpc": "2.0"`.
///
/// ```rust
/// use coding_adventures_json_rpc::message::{message_to_value, Message, Response};
///
/// let resp = Response {
///     id: serde_json::json!(1),
///     result: Some(serde_json::json!({"ok": true})),
///     error: None,
/// };
/// let val = message_to_value(&Message::Response(resp));
/// assert_eq!(val["jsonrpc"], "2.0");
/// assert_eq!(val["id"], 1);
/// ```
pub fn message_to_value(message: &Message) -> Value {
    match message {
        Message::Request(req) => {
            let mut map = serde_json::Map::new();
            map.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
            map.insert("id".to_string(), req.id.clone());
            map.insert("method".to_string(), Value::String(req.method.clone()));
            if let Some(params) = &req.params {
                map.insert("params".to_string(), params.clone());
            }
            Value::Object(map)
        }

        Message::Response(resp) => {
            let mut map = serde_json::Map::new();
            map.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
            map.insert("id".to_string(), resp.id.clone());
            if let Some(error) = &resp.error {
                // Error response — include the error object.
                map.insert(
                    "error".to_string(),
                    serde_json::to_value(error).unwrap_or(Value::Null),
                );
            } else {
                // Success response — include the result (may be null).
                map.insert(
                    "result".to_string(),
                    resp.result.clone().unwrap_or(Value::Null),
                );
            }
            Value::Object(map)
        }

        Message::Notification(notif) => {
            let mut map = serde_json::Map::new();
            map.insert("jsonrpc".to_string(), Value::String("2.0".to_string()));
            map.insert("method".to_string(), Value::String(notif.method.clone()));
            if let Some(params) = &notif.params {
                map.insert("params".to_string(), params.clone());
            }
            Value::Object(map)
        }
    }
}

// ---------------------------------------------------------------------------
// Serialise / Deserialise helpers
// ---------------------------------------------------------------------------
//
// We provide explicit Serialize/Deserialize impls by going through
// message_to_value / parse_message rather than using serde's derive, because
// the disambiguation logic (presence of "id") cannot be expressed with serde's
// built-in attributes.

impl Serialize for Message {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        message_to_value(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let bytes = serde_json::to_vec(&value).map_err(serde::de::Error::custom)?;
        parse_message(&bytes).map_err(|e| serde::de::Error::custom(e.message))
    }
}
