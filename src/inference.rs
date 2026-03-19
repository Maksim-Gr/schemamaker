use crate::schema::{Column, ColumnType, InferredSchema};
use serde_json::Value;
use std::collections::HashMap;

pub fn infer_schema(content: &str, table_name: &str) -> Result<InferredSchema, String> {
    let mut field_order: Vec<String> = Vec::new();
    let mut field_types: HashMap<String, ColumnType> = HashMap::new();
    let mut record_count = 0;

    for (line_number, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|e| format!("Error parsing line {}: {}", line_number + 1, e))?;
        let obj = value.as_object().ok_or_else(|| {
            format!(
                "line {}: expected a JSON object, got smt else",
                line_number + 1
            )
        })?;

        for (key, value) in obj {
            let inferred = infer_type(value);
            if let Some(existing) = field_types.get(key) {
                field_types.insert(key.clone(), merge_types(existing, &inferred));
            } else {
                field_order.push(key.clone());
                field_types.insert(key.clone(), inferred);
            }
        }
        record_count += 1;
    }
    if record_count == 0 {
        return Err("input file is empty or contains no JSON objects".to_string());
    }

    let columns: Vec<Column> = field_order
        .into_iter()
        .map(|name| {
            let ch_type = field_types.remove(&name).unwrap();
            Column { name, ch_type }
        })
        .collect();
    eprintln!(
        "Inferred {} columns from {} records",
        columns.len(),
        record_count
    );
    Ok(InferredSchema {
        table_name: table_name.to_string(),
        columns,
    })
}

fn infer_type(val: &Value) -> ColumnType {
    match val {
        Value::Bool(_) => ColumnType::Bool,
        Value::String(_) => ColumnType::String,
        Value::Number(n) => {
            if n.as_i64().is_some() || n.as_u64().is_some() {
                ColumnType::Int64
            } else {
                ColumnType::Float64
            }
        }
        _ => ColumnType::String,
    }
}
fn merge_types(a: &ColumnType, b: &ColumnType) -> ColumnType {
    if a == b {
        return a.clone();
    }
    match (a, b) {
        (ColumnType::Float64, ColumnType::Int64) | (ColumnType::Int64, ColumnType::Float64) => {
            ColumnType::Float64
        }
        _ => ColumnType::String,
    }
}
