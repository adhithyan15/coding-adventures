//! Isolated Flash runtime environment and CWE-1321 prototype pollution defense.

#![deny(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use vm_core::value::Value;

use crate::error::LifterError;

/// Flash runtime environment protecting against CWE-1321 prototype pollution.
#[derive(Debug, Clone)]
pub struct AvmEnvironment {
    frozen_prototypes: HashSet<String>,
    variables: Arc<Mutex<HashMap<String, Value>>>,
    object_store: Arc<Mutex<HashMap<usize, HashMap<String, Value>>>>,
    next_object_id: Arc<Mutex<usize>>,
    action_log: Arc<Mutex<Vec<String>>>,
    output: Arc<Mutex<String>>,
}

impl Default for AvmEnvironment {
    fn default() -> Self {
        Self::env_new()
    }
}

impl AvmEnvironment {
    /// Create a new isolated runtime environment with frozen native built-in prototypes.
    pub fn env_new() -> Self {
        let mut frozen = HashSet::new();
        // Register core ActionScript native classes whose prototypes must remain immutable
        frozen.insert("Object".to_string());
        frozen.insert("Object.prototype".to_string());
        frozen.insert("Function".to_string());
        frozen.insert("Function.prototype".to_string());
        frozen.insert("Array".to_string());
        frozen.insert("Array.prototype".to_string());
        frozen.insert("String".to_string());
        frozen.insert("String.prototype".to_string());
        frozen.insert("Number".to_string());
        frozen.insert("Number.prototype".to_string());
        frozen.insert("Boolean".to_string());
        frozen.insert("Boolean.prototype".to_string());
        frozen.insert("Math".to_string());
        frozen.insert("MovieClip".to_string());
        frozen.insert("MovieClip.prototype".to_string());

        let mut store = HashMap::new();
        // Pre-populate frozen native prototype objects in the store
        let mut obj_proto = HashMap::new();
        obj_proto.insert("__class__".to_string(), Value::str("Object"));
        store.insert(1, obj_proto);

        Self {
            frozen_prototypes: frozen,
            variables: Arc::new(Mutex::new(HashMap::new())),
            object_store: Arc::new(Mutex::new(store)),
            next_object_id: Arc::new(Mutex::new(2)),
            action_log: Arc::new(Mutex::new(Vec::new())),
            output: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Check if target object or property write constitutes prototype pollution (CWE-1321).
    pub fn env_is_prototype_pollution(&self, target_name: &str, prop_name: &str) -> bool {
        if prop_name == "__proto__" || prop_name == "prototype" || prop_name == "constructor" {
            return true;
        }
        if self.frozen_prototypes.contains(target_name) {
            return true;
        }
        let full_path = format!("{}.{}", target_name, prop_name);
        if self.frozen_prototypes.contains(&full_path) {
            return true;
        }
        false
    }

    /// Guard property assignment against CWE-1321.
    pub fn env_set_member(&self, obj_id: usize, prop: &str, val: Value) -> Result<(), LifterError> {
        if prop == "__proto__" || prop == "constructor" {
            return Err(LifterError::PrototypePollutionBlocked(format!(
                "attempted modification of forbidden prototype property: {}",
                prop
            )));
        }

        let mut store = self.object_store.lock().unwrap();
        if let Some(map) = store.get_mut(&obj_id) {
            if let Some(cls) = map.get("__class__") {
                let cls_str = cls.to_string();
                if self.frozen_prototypes.contains(&cls_str) {
                    return Err(LifterError::PrototypePollutionBlocked(format!(
                        "attempted modification of frozen built-in prototype: {}",
                        cls_str
                    )));
                }
            }
            map.insert(prop.to_string(), val);
        } else {
            let mut map = HashMap::new();
            map.insert(prop.to_string(), val);
            store.insert(obj_id, map);
        }
        Ok(())
    }

    /// Read object property member.
    pub fn env_get_member(&self, obj_id: usize, prop: &str) -> Value {
        let store = self.object_store.lock().unwrap();
        if let Some(map) = store.get(&obj_id) {
            if let Some(val) = map.get(prop) {
                return val.clone();
            }
        }
        Value::Null
    }

    /// Set a scoped variable, checking for prototype pollution if global/prototype is targeted.
    pub fn env_set_variable(&self, name: &str, val: Value) -> Result<(), LifterError> {
        if self.frozen_prototypes.contains(name) {
            return Err(LifterError::PrototypePollutionBlocked(format!(
                "attempted overwrite of frozen built-in: {}",
                name
            )));
        }
        if name.starts_with("_global.Object.prototype")
            || name.starts_with("_global.Function.prototype")
            || name.contains("__proto__")
        {
            return Err(LifterError::PrototypePollutionBlocked(format!(
                "attempted prototype pollution via variable path: {}",
                name
            )));
        }
        let mut vars = self.variables.lock().unwrap();
        vars.insert(name.to_string(), val);
        Ok(())
    }

    /// Read a scoped variable.
    pub fn env_get_variable(&self, name: &str) -> Value {
        let vars = self.variables.lock().unwrap();
        vars.get(name).cloned().unwrap_or(Value::Null)
    }

    /// Allocate a new object instance in the environment.
    pub fn env_new_object(&self, class_name: &str) -> usize {
        let mut id_lock = self.next_object_id.lock().unwrap();
        let id = *id_lock;
        *id_lock += 1;

        let mut map = HashMap::new();
        map.insert("__class__".to_string(), Value::str(class_name));
        self.object_store.lock().unwrap().insert(id, map);
        id
    }

    /// Append a message to the trace output buffer.
    pub fn env_append_output(&self, msg: &str) {
        let mut out = self.output.lock().unwrap();
        out.push_str(msg);
        out.push('\n');
    }

    /// Get captured trace output.
    pub fn env_get_output(&self) -> String {
        self.output.lock().unwrap().clone()
    }

    /// Log a timeline action verb (e.g. "stop", "play", "gotoFrame(5)").
    pub fn env_log_action(&self, act: &str) {
        self.action_log.lock().unwrap().push(act.to_string());
    }

    /// Get logged timeline actions.
    pub fn env_get_action_log(&self) -> Vec<String> {
        self.action_log.lock().unwrap().clone()
    }
}
