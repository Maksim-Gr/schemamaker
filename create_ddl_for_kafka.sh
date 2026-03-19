#!/bin/bash
set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <input_json_file>"
  exit 1
fi

INPUT_JSON="$1"

if [ ! -f "$INPUT_JSON" ]; then
  echo "Error: File '$INPUT_JSON' not found"
  exit 1
fi

BASENAME=$(basename "$INPUT_JSON" .json)
FINAL_OUTPUT="${BASENAME}.sql"
TEMP_FILE=$(mktemp /tmp/table_def_XXXXXX.sql)

trap 'rm -f "$TEMP_FILE"' EXIT

echo "Generating DDL from $INPUT_JSON..."

./clickhouse.0 local -q "
CREATE TABLE ${BASENAME}_stat ENGINE = MergeTree ORDER BY tuple() AS
SELECT * FROM file('$INPUT_JSON', 'JSONEachRow')
SETTINGS schema_inference_make_columns_nullable = 1;

SHOW CREATE TABLE ${BASENAME}_stat;
" \
| sed 's/\\n/\n/g; s/\\t/\t/g; s/\\//g' \
| awk '
{
  line = $0
  match($0, /^[ \t]*/)
  indent = substr($0, RSTART, RLENGTH)

  trimmed = line
  sub(/^[ \t]+/, "", trimmed)

  # Skip the ENGINE, ORDER BY and SETTINGS lines from the temporary table
  if (trimmed ~ /^ENGINE = MergeTree/ || trimmed ~ /^ORDER BY tuple\(\)/ || trimmed ~ /^SETTINGS /) {
    next
  }

  if (trimmed == "" || trimmed ~ /^[()]*$/) {
    print line
    next
  }

  split(trimmed, parts, /[ \t]+/)
  col = parts[1]
  rest = substr(trimmed, length(col) + 2)

  if (col ~ /^`.*`$/) {
    print line
  } else {
    print indent "`" col "` " rest
  }
}
' > "$TEMP_FILE"

(
cat << 'EOF'
CREATE TABLE IF NOT EXISTS streams.{table_name} ON CLUSTER aip_clickhouse_datalake
(
    `message` String
)
    ENGINE = Kafka(aip_kafka) SETTINGS kafka_topic_list =
    'private.{environment}.{table_name}.v1', kafka_group_name =
    'clickhouse-{environment}xdcl-{table_name}-shard-1', kafka_format = 'RawBLOB';

CREATE TABLE IF NOT EXISTS raw.{table_name} ON CLUSTER aip_clickhouse_datalake
(
    `message`       String,
    `_key`          String,
    `_offset`       UInt64,
    `_partition`    UInt64,
    `_timestamp_ms` DateTime64(3),
    `_topic`        LowCardinality(String),
    `_row_created`  DateTime DEFAULT nowInBlock()
)
    ENGINE = ReplicatedMergeTree('/clickhouse/{cluster}/tables/raw/{table_name}/{shard}',
                                '{replica}')
    PARTITION BY toYYYYMM(_row_created)
    ORDER BY _row_created
    SETTINGS index_granularity = 8192;

EOF

echo "CREATE TABLE IF NOT EXISTS datalake.{table_name} ON CLUSTER aip_clickhouse_datalake"
awk '
NR == 1 { next }
/^\)$/ {
    if (prev !~ /,$/) prev = prev ","
    print prev
    print "    `_timestamp_ms` DateTime64(3),"
    print "    `_topic` LowCardinality(String),"
    print "    `_row_created` DateTime DEFAULT nowInBlock()"
    print ")"
    prev = ""
    next
}
NR > 1 && prev != "" { print prev }
{ prev = $0 }
END { if (prev != "") print prev }
' "$TEMP_FILE"
echo "    ENGINE = ReplicatedMergeTree('/clickhouse/{cluster}/tables/datalake/{table_name}/{shard}',
                                '{replica}')
    PARTITION BY toYYYYMM(_timestamp_ms)
    ORDER BY _timestamp_ms
    SETTINGS index_granularity = 8192;"

cat << 'EOF'

CREATE MATERIALIZED VIEW IF NOT EXISTS raw.{table_name}_mv
    ON CLUSTER aip_clickhouse_datalake TO datalake.{table_name} AS
SELECT *
FROM (
    SELECT
EOF

awk 'BEGIN {
    sep = "";
    in_type = 0;
    type = "";
    current_field = "";
}
/^    `/ {
    if (in_type) {
        type = type prev_line
        in_type = 0
        gsub(/,$/, "", type)
        gsub(/"/, "\\\"", type)
        gsub(/^[ \t]+/, "", type)
        gsub(/`([^`]+)`[ \t]*/, "", type)
        gsub(/^[ \t]+/, "", type)

        type_formatted = type
        gsub(/[ \t]+/, " ", type_formatted)
        sub(/^[ \t]+/, "", type_formatted)
        printf "%s        JSONExtract(message, '\''" current_field "'\'', '\''" type_formatted "'\'')" " AS " current_field, sep
        sep = ",\n"
    }

    current_field = $1
    gsub(/`/, "", current_field)
    if (current_field != "_timestamp_ms" && current_field != "_topic" && current_field != "_row_created") {
        in_type = 1
        type = ""
    }
}
/^    [^`]/ {
    if (in_type) {
        type = type prev_line
    }
}
/^\)$/ { next }
{ prev_line = $0 }
END {
    if (in_type) {
        type = type prev_line
        gsub(/,$/, "", type)
        gsub(/"/, "\\\"", type)
        gsub(/^[ \t]+/, "", type)
        gsub(/`([^`]+)`[ \t]*/, "", type)
        gsub(/^[ \t]+/, "", type)

        type_formatted = type
        gsub(/[ \t]+/, " ", type_formatted)
        sub(/^[ \t]+/, "", type_formatted)
        printf "%s        JSONExtract(message, '\''" current_field "'\'', '\''" type_formatted "'\'')" " AS " current_field, sep
    }
}' "$TEMP_FILE"
printf ",\n"
cat << 'EOF'
        `_timestamp_ms`,
        `_topic`,
        `_row_created`
    FROM raw.{table_name}
) SETTINGS cast_keep_nullable = 1;

CREATE MATERIALIZED VIEW IF NOT EXISTS streams.{table_name}_mv
    ON CLUSTER aip_clickhouse_datalake
    TO raw.{table_name} (
        `message` String,
        `_key` String,
        `_offset` UInt64,
        `_partition` UInt64,
        `_timestamp_ms` DateTime64(3),
        `_topic` LowCardinality(String),
        `_row_created` DateTime
    ) AS
SELECT
    message,
    _key,
    _offset,
    _partition,
    assumeNotNull(_timestamp_ms) AS _timestamp_ms,
    _topic,
    nowInBlock() AS _row_created
FROM streams.{table_name};
EOF
) | sed "s/{table_name}/$BASENAME/g" > "$FINAL_OUTPUT"

echo "Done! Final DDL saved to $FINAL_OUTPUT"
