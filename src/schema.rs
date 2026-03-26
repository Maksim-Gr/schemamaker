#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColumnType {
    String,
    Int64,
    Float64,
    Bool,
}

impl ColumnType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColumnType::String => "String",
            ColumnType::Int64 => "Int64",
            ColumnType::Float64 => "Float64",
            ColumnType::Bool => "Bool",
        }
    }

    pub fn as_nullable_str(&self) -> &'static str {
        match self {
            ColumnType::String => "Nullable(String)",
            ColumnType::Int64 => "Nullable(Int64)",
            ColumnType::Float64 => "Nullable(Float64)",
            ColumnType::Bool => "Nullable(Bool)",
        }
    }

    pub fn as_ch_str(&self, nullable: bool) -> &'static str {
        if nullable {
            self.as_nullable_str()
        } else {
            self.as_str()
        }
    }
}

pub struct Column {
    pub name: String,
    pub ch_type: ColumnType,
    pub nullable: bool,
}

pub struct InferredSchema {
    pub table_name: String,
    pub columns: Vec<Column>,
}
