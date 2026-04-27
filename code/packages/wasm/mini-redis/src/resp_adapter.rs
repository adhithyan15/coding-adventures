use in_memory_data_store::EngineResponse;
use resp_protocol::RespValue;

pub fn engine_response_to_resp(response: EngineResponse) -> RespValue {
    match response {
        EngineResponse::SimpleString(s) => RespValue::SimpleString(s),
        EngineResponse::Error(e) => RespValue::Error(resp_protocol::RespError::new(e)),
        EngineResponse::Integer(i) => RespValue::Integer(i),
        EngineResponse::BulkString(b) => RespValue::BulkString(b),
        EngineResponse::Array(a) => RespValue::Array(a.map(|arr| {
            arr.into_iter().map(engine_response_to_resp).collect()
        })),
    }
}

pub fn command_frame_from_resp(value: RespValue) -> Option<in_memory_data_store::CommandFrame> {
    match value {
        RespValue::Array(Some(values)) => {
            let mut parts = Vec::with_capacity(values.len());
            for item in values {
                match item {
                    RespValue::BulkString(Some(bytes)) => parts.push(bytes),
                    RespValue::SimpleString(text) => parts.push(text.into_bytes()),
                    RespValue::Integer(n) => parts.push(n.to_string().into_bytes()),
                    _ => return None,
                }
            }
            in_memory_data_store::CommandFrame::from_parts(parts)
        }
        _ => None,
    }
}
