#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    String,
    Int64,
    Float64,
    Bool,
}

impl ColumnType {
    pub fn as_nullable_str(&self) -> &'static str {
        match self {
            ColumnType::String => "Nullable(String)",
            ColumnType::Int64 => "Nullable(Int64)",
            ColumnType::Float64 => "Nullable(Float64)",
            ColumnType::Bool => "Nullable(Bool)",
        }
    }
}

pub struct Column {
    pub name: String,
    pub ch_type: ColumnType,
}

pub struct InferredSchema {
    pub table_name: String,
    pub columns: Vec<Column>,
}
