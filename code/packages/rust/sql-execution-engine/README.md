# sql-execution-engine (Rust)

A **SELECT-only SQL execution engine** that executes parsed SQL queries against
any pluggable data source.

## Usage

```rust
use coding_adventures_sql_execution_engine::{execute, DataSource, ExecutionError, SqlValue, SqlPrimitive};
use std::collections::HashMap;

struct MySource;

impl DataSource for MySource {
    fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError> {
        match table_name {
            "users" => Ok(vec!["id".to_string(), "name".to_string()]),
            _ => Err(ExecutionError::TableNotFound(table_name.to_string())),
        }
    }

    fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, SqlValue>>, ExecutionError> {
        match table_name {
            "users" => Ok(vec![{
                let mut row = HashMap::new();
                row.insert("id".to_string(), Some(SqlPrimitive::Int(1)));
                row.insert("name".to_string(), Some(SqlPrimitive::Text("Alice".to_string())));
                row
            }]),
            _ => Err(ExecutionError::TableNotFound(table_name.to_string())),
        }
    }
}

let result = execute("SELECT name FROM users", &MySource).unwrap();
println!("{:?}", result.columns); // ["name"]
```
