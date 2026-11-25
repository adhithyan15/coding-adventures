//! Virtual machine runner and Flash runtime builtins bridge.

#![deny(unsafe_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use interpreter_ir::module::IIRModule;
use vm_core::value::Value;
use vm_core::VMCore;

use crate::error::AsCompilerError;

fn is_forbidden_prototype_prop(prop: &str) -> bool {
    let lower = prop.to_ascii_lowercase();
    lower == "__proto__" || lower == "prototype" || lower == "constructor"
}

fn format_value_str(v: &Value) -> String {
    match v {
        Value::Str(bytes) => String::from_utf8_lossy(bytes).to_string(),
        Value::Null => "null".to_string(),
        Value::Int(n) => n.to_string(),
        Value::Float(f) => {
            if f.fract() == 0.0 && *f >= (i64::MIN as f64) && *f <= (i64::MAX as f64) {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

/// Flash runtime environment and VM execution harness.
pub struct AsVmRunner {
    output: Arc<Mutex<String>>,
    action_log: Arc<Mutex<Vec<String>>>,
    object_store: Arc<Mutex<HashMap<usize, HashMap<String, Value>>>>,
    next_obj_id: Arc<Mutex<usize>>,
}

impl Default for AsVmRunner {
    fn default() -> Self {
        Self::runner_new()
    }
}

impl AsVmRunner {
    /// Create a new Flash VM runner.
    pub fn runner_new() -> Self {
        Self {
            output: Arc::new(Mutex::new(String::new())),
            action_log: Arc::new(Mutex::new(Vec::new())),
            object_store: Arc::new(Mutex::new(HashMap::new())),
            next_obj_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Execute an IIR module and return the result of `main`.
    pub fn runner_execute(&mut self, module: &IIRModule) -> Result<Value, AsCompilerError> {
        let mut vm = VMCore::new();
        vm.max_instructions = Some(1_000_000);
        self.runner_register_flash_builtins(&mut vm)?;

        let mut mod_clone = module.clone();
        let entry = mod_clone
            .entry_point
            .clone()
            .unwrap_or_else(|| "main".to_string());
        let res = vm
            .execute(&mut mod_clone, &entry, &[])
            .map_err(|e| AsCompilerError::ExecutionError(format!("VM runtime error: {:?}", e)))?;
        Ok(res.unwrap_or(Value::Null))
    }

    /// Retrieve captured trace/print output.
    pub fn runner_output(&self) -> String {
        self.output
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Retrieve logged Flash timeline actions.
    pub fn runner_action_log(&self) -> Vec<String> {
        self.action_log
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn runner_register_flash_builtins(&self, vm: &mut VMCore) -> Result<(), AsCompilerError> {
        let out_ref = Arc::clone(&self.output);
        let log_ref = Arc::clone(&self.action_log);
        let store_ref = Arc::clone(&self.object_store);
        let id_ref = Arc::clone(&self.next_obj_id);

        let builtins = vm.builtins_mut();

        // flash_trace and trace
        let out_clone = Arc::clone(&out_ref);
        builtins.register("flash_trace", move |args| {
            let s = if let Some(v) = args.first() {
                format_value_str(v)
            } else {
                String::new()
            };
            let mut o = out_clone.lock().unwrap_or_else(|e| e.into_inner());
            o.push_str(&s);
            o.push('\n');
            Ok(Value::Null)
        });

        let out_clone2 = Arc::clone(&out_ref);
        builtins.register("trace", move |args| {
            let s = if let Some(v) = args.first() {
                format_value_str(v)
            } else {
                String::new()
            };
            let mut o = out_clone2.lock().unwrap_or_else(|e| e.into_inner());
            o.push_str(&s);
            o.push('\n');
            Ok(Value::Null)
        });

        // Flash Timeline Verbs
        let log_clone = Arc::clone(&log_ref);
        builtins.register("flash_stop", move |_| {
            log_clone
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push("stop".to_string());
            Ok(Value::Null)
        });

        let log_clone2 = Arc::clone(&log_ref);
        builtins.register("flash_play", move |_| {
            log_clone2
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push("play".to_string());
            Ok(Value::Null)
        });

        let log_clone3 = Arc::clone(&log_ref);
        builtins.register("flash_goto_and_play", move |args| {
            let f = args.first().map_or("0".to_string(), format_value_str);
            log_clone3
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("gotoAndPlay({})", f));
            Ok(Value::Null)
        });

        let log_clone4 = Arc::clone(&log_ref);
        builtins.register("flash_goto_and_stop", move |args| {
            let f = args.first().map_or("0".to_string(), format_value_str);
            log_clone4
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("gotoAndStop({})", f));
            Ok(Value::Null)
        });

        let log_clone5 = Arc::clone(&log_ref);
        builtins.register("flash_tell_target", move |args| {
            let t = args.first().map_or("".to_string(), format_value_str);
            log_clone5
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("tellTarget({})", t));
            Ok(Value::Null)
        });

        let log_clone6 = Arc::clone(&log_ref);
        builtins.register("flash_end_tell_target", move |_| {
            log_clone6
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push("endTellTarget".to_string());
            Ok(Value::Null)
        });

        let log_clone7 = Arc::clone(&log_ref);
        builtins.register("flash_call", move |args| {
            let f = args.first().map_or("".to_string(), format_value_str);
            log_clone7
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("call({})", f));
            Ok(Value::Null)
        });

        // Flash Paths
        builtins.register("flash_get_root", move |_| Ok(Value::str("_root")));
        builtins.register("flash_get_parent", move |_| Ok(Value::str("_parent")));
        builtins.register("flash_get_global", move |_| Ok(Value::str("_global")));
        builtins.register("flash_get_this", move |_| Ok(Value::str("this")));

        // Flash Object System
        let store_c1 = Arc::clone(&store_ref);
        let id_c1 = Arc::clone(&id_ref);
        builtins.register("flash_new_object", move |_| {
            let mut id_lock = id_c1.lock().unwrap_or_else(|e| e.into_inner());
            let id = *id_lock;
            *id_lock += 1;
            store_c1
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id, HashMap::new());
            Ok(Value::Int(id as i64))
        });

        let store_c2 = Arc::clone(&store_ref);
        let id_c2 = Arc::clone(&id_ref);
        builtins.register("flash_new_instance", move |args| {
            let cls = args.first().map_or("Object".to_string(), format_value_str);
            let mut id_lock = id_c2.lock().unwrap_or_else(|e| e.into_inner());
            let id = *id_lock;
            *id_lock += 1;
            let mut map = HashMap::new();
            map.insert("__class__".to_string(), Value::str(cls));
            store_c2
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(id, map);
            Ok(Value::Int(id as i64))
        });

        let store_c3 = Arc::clone(&store_ref);
        builtins.register("flash_set_property", move |args| {
            if args.len() >= 3 {
                let obj_id = args[0].as_i64().unwrap_or(0) as usize;
                let prop = format_value_str(&args[1]);
                if is_forbidden_prototype_prop(&prop) {
                    return Err(vm_core::VMError::Custom(format!(
                        "CWE-1321 prototype pollution blocked on property '{}'",
                        prop
                    )));
                }
                let val = args[2].clone();
                let mut s = store_c3.lock().unwrap_or_else(|e| e.into_inner());
                let entry = s.entry(obj_id).or_default();
                entry.insert(prop, val);
            }
            Ok(Value::Null)
        });

        let store_c4 = Arc::clone(&store_ref);
        builtins.register("flash_get_property", move |args| {
            if args.len() >= 2 {
                let obj_id = args[0].as_i64().unwrap_or(0) as usize;
                let prop = format_value_str(&args[1]);
                if is_forbidden_prototype_prop(&prop) {
                    return Err(vm_core::VMError::Custom(format!(
                        "CWE-1321 prototype access blocked on property '{}'",
                        prop
                    )));
                }
                let s = store_c4.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(map) = s.get(&obj_id) {
                    if let Some(val) = map.get(&prop) {
                        return Ok(val.clone());
                    }
                }
            }
            Ok(Value::Null)
        });

        let store_c5 = Arc::clone(&store_ref);
        builtins.register("flash_invoke_method", move |args| {
            if args.len() >= 2 {
                let obj_id = args[0].as_i64().unwrap_or(0) as usize;
                let prop = format_value_str(&args[1]);
                if is_forbidden_prototype_prop(&prop) {
                    return Err(vm_core::VMError::Custom(format!(
                        "CWE-1321 prototype invocation blocked on property '{}'",
                        prop
                    )));
                }
                let s = store_c5.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(map) = s.get(&obj_id) {
                    if let Some(val) = map.get(&prop) {
                        return Ok(val.clone());
                    }
                }
            }
            Ok(Value::Null)
        });

        // flash_typeof
        builtins.register("flash_typeof", move |args| {
            let t = match args.first() {
                Some(Value::Int(_)) | Some(Value::Float(_)) => "number",
                Some(Value::Str(_)) => "string",
                Some(Value::Bool(_)) => "boolean",
                Some(Value::Null) => "undefined",
                _ => "object",
            };
            Ok(Value::str(t))
        });

        // flash_is and flash_as
        builtins.register("flash_is", move |args| {
            if args.len() < 2 {
                return Ok(Value::Bool(false));
            }
            let target_type = format_value_str(&args[1]);
            let is_match = matches!(
                (args[0].clone(), target_type.as_str()),
                (Value::Int(_), "int")
                    | (Value::Int(_), "uint")
                    | (Value::Int(_), "Number")
                    | (Value::Float(_), "Number")
                    | (Value::Str(_), "String")
                    | (Value::Bool(_), "Boolean")
                    | (Value::Int(_), "Object")
            );
            Ok(Value::Bool(is_match))
        });

        builtins.register("flash_as", move |args| {
            if args.len() < 2 {
                return Ok(Value::Null);
            }
            let target_type = format_value_str(&args[1]);
            let is_match = matches!(
                (args[0].clone(), target_type.as_str()),
                (Value::Int(_), "int")
                    | (Value::Int(_), "uint")
                    | (Value::Int(_), "Number")
                    | (Value::Float(_), "Number")
                    | (Value::Str(_), "String")
                    | (Value::Bool(_), "Boolean")
            );
            if is_match {
                Ok(args[0].clone())
            } else {
                Ok(Value::Null)
            }
        });

        // flash_eval
        builtins.register("flash_eval", move |args| {
            if let Some(val) = args.first() {
                Ok(val.clone())
            } else {
                Ok(Value::Null)
            }
        });

        // flash_throw
        let log_clone8 = Arc::clone(&log_ref);
        builtins.register("flash_throw", move |args| {
            let msg = args.first().map_or("".to_string(), format_value_str);
            log_clone8
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("throw({})", msg));
            Ok(Value::Null)
        });

        // flash_register_clip_event
        let log_clone9 = Arc::clone(&log_ref);
        builtins.register("flash_register_clip_event", move |args| {
            let ev = args.first().map_or("".to_string(), format_value_str);
            let handler = args.get(1).map_or("".to_string(), format_value_str);
            log_clone9
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(format!("onClipEvent({}, {})", ev, handler));
            Ok(Value::Null)
        });

        Ok(())
    }
}
