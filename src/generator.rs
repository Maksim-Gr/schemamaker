use crate::schema::InferredSchema;

pub struct Generator<'a> {
    schema: &'a InferredSchema,
    cluster: String,
    kafka: String,
}

impl<'a> Generator<'a> {
    pub fn new(schema: &'a InferredSchema, cluster: String, kafka: String) -> Self {
        Generator {
            schema,
            cluster,
            kafka,
        }
    }

    pub fn generate_up(&self) -> String {
        vec![
            self.streams_table(),
            self.raw_table(),
            self.datalake_table(),
            self.raw_mv(),
            self.streams_mv(),
        ]
        .join("\n\n")
    }

    pub fn generate_down(&self) -> String {
        let t = &self.schema.table_name;
        let c = &self.cluster;
        format!(
            "DROP MATERIALIZED VIEW IF EXISTS streams.{t}_mv ON CLUSTER {c};\n\
             DROP MATERIALIZED VIEW IF EXISTS raw.{t}_mv ON CLUSTER {c};\n\
             DROP TABLE IF EXISTS datalake.{t} ON CLUSTER {c};\n\
             DROP TABLE IF EXISTS raw.{t} ON CLUSTER {c};\n\
             DROP TABLE IF EXISTS streams.{t} ON CLUSTER {c};"
        )
    }

    fn streams_table(&self) -> String {
        let t = &self.schema.table_name;
        let c = &self.cluster;
        let k = &self.kafka;
        format!(
            "CREATE TABLE IF NOT EXISTS streams.{t} ON CLUSTER {c}\n\
             (\n\
             \t`message` String\n\
             )\n\
             \tENGINE = Kafka({k}) SETTINGS kafka_topic_list =\n\
             \t'private.{{environment}}.{t}.v1', kafka_group_name =\n\
             \t'clickhouse-{{environment}}xdcl-{t}-shard-1', kafka_format = 'RawBLOB';"
        )
    }

    fn raw_table(&self) -> String {
        let t = &self.schema.table_name;
        let c = &self.cluster;
        format!(
            "CREATE TABLE IF NOT EXISTS raw.{t} ON CLUSTER {c}\n\
             (\n\
             \t`message`       String,\n\
             \t`_key`          String,\n\
             \t`_offset`       UInt64,\n\
             \t`_partition`    UInt64,\n\
             \t`_timestamp_ms` DateTime64(3),\n\
             \t`_topic`        LowCardinality(String),\n\
             \t`_row_created`  DateTime DEFAULT nowInBlock()\n\
             )\n\
             \tENGINE = ReplicatedMergeTree('/clickhouse/{{cluster}}/tables/raw/{t}/{{shard}}', '{{replica}}')\n\
             \tPARTITION BY toYYYYMM(_row_created)\n\
             \tORDER BY _row_created\n\
             \tSETTINGS index_granularity = 8192;"
        )
    }

    fn datalake_table(&self) -> String {
        let t = &self.schema.table_name;
        let c = &self.cluster;
        let cols: String = self
            .schema
            .columns
            .iter()
            .map(|col| {
                format!(
                    "\t`{}` {},\n",
                    col.name,
                    col.ch_type.as_ch_str(col.nullable)
                )
            })
            .collect();
        format!(
            "CREATE TABLE IF NOT EXISTS datalake.{t} ON CLUSTER {c}\n\
             (\n\
             {cols}\
             \t`_timestamp_ms` DateTime64(3),\n\
             \t`_topic`        LowCardinality(String),\n\
             \t`_row_created`  DateTime DEFAULT nowInBlock()\n\
             )\n\
             \tENGINE = ReplicatedMergeTree('/clickhouse/{{cluster}}/tables/datalake/{t}/{{shard}}', '{{replica}}')\n\
             \tPARTITION BY toYYYYMM(_timestamp_ms)\n\
             \tORDER BY _timestamp_ms\n\
             \tSETTINGS index_granularity = 8192;"
        )
    }

    fn raw_mv(&self) -> String {
        let t = &self.schema.table_name;
        let c = &self.cluster;
        let extracts: String = self
            .schema
            .columns
            .iter()
            .map(|col| {
                format!(
                    "\t\tJSONExtract(message, '{}', '{}') AS {},\n",
                    col.name,
                    col.ch_type.as_ch_str(col.nullable),
                    col.name
                )
            })
            .collect();
        format!(
            "CREATE MATERIALIZED VIEW IF NOT EXISTS raw.{t}_mv\n\
             \tON CLUSTER {c} TO datalake.{t} AS\n\
             SELECT * FROM (\n\
             \tSELECT\n\
             {extracts}\
             \t\t`_timestamp_ms`,\n\
             \t\t`_topic`,\n\
             \t\t`_row_created`\n\
             \tFROM raw.{t}\n\
             ) SETTINGS cast_keep_nullable = 1;"
        )
    }

    fn streams_mv(&self) -> String {
        let t = &self.schema.table_name;
        let c = &self.cluster;
        format!(
            "CREATE MATERIALIZED VIEW IF NOT EXISTS streams.{t}_mv\n\
             \tON CLUSTER {c}\n\
             \tTO raw.{t} (\n\
             \t\t`message`        String,\n\
             \t\t`_key`           String,\n\
             \t\t`_offset`        UInt64,\n\
             \t\t`_partition`     UInt64,\n\
             \t\t`_timestamp_ms`  DateTime64(3),\n\
             \t\t`_topic`         LowCardinality(String),\n\
             \t\t`_row_created`   DateTime\n\
             \t) AS\n\
             SELECT\n\
             \tmessage,\n\
             \t_key,\n\
             \t_offset,\n\
             \t_partition,\n\
             \tassumeNotNull(_timestamp_ms) AS _timestamp_ms,\n\
             \t_topic,\n\
             \tnowInBlock() AS _row_created\n\
             FROM streams.{t};"
        )
    }
}
