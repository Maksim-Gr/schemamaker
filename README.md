# schemamaker

Generate ClickHouse migration SQL from a JSON file. Replaces the `create_ddl_for_kafka.sh` shell script without requiring a ClickHouse binary.

## Install

Download the binary for your platform from the [v0.1.1-beta release](https://github.com/Maksim-Gr/schemamaker/releases/tag/v0.1.1-beta).

**macOS (Apple Silicon)**
```bash
curl -L https://github.com/Maksim-Gr/schemamaker/releases/download/v0.1.1-beta/schemamaker-macos-arm64.tar.gz | tar -xz
chmod +x schemamaker && mv schemamaker /usr/local/bin/schemamaker
```

**macOS (Intel)**
```bash
curl -L https://github.com/Maksim-Gr/schemamaker/releases/download/v0.1.1-beta/schemamaker-macos-x86_64.tar.gz | tar -xz
chmod +x schemamaker && mv schemamaker /usr/local/bin/schemamaker
```

**Linux (x86_64)**
```bash
curl -L https://github.com/Maksim-Gr/schemamaker/releases/download/v0.1.1-beta/schemamaker-linux-x86_64.tar.gz | tar -xz
chmod +x schemamaker && mv schemamaker /usr/local/bin/schemamaker
```

Verify:
```bash
schemamaker --version
```

## Build from source

```bash
cargo build --release
```

The binary is at `./target/release/schemamaker`.

## Usage

```bash
schemamaker [OPTIONS] <INPUT>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Path to a JSON file (NDJSON, pretty-printed, or JSON array) |

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-n, --name <NAME>` | input filename stem | Override the table name |
| `-c, --cluster <CLUSTER>` | `clickhouse_datalake` | ClickHouse cluster name |
| `-k, --kafka <KAFKA>` | `kafka` | Kafka collection name |
| `-o, --output-dir <DIR>` | `.` | Output directory for generated SQL files |

## Examples

```bash
# Infer schema from video_events.json, use all defaults
schemamaker video_events.json
# → video_events_up.sql
# → video_events_down.sql

# Override table name
schemamaker video_events.json -n my_table

# Override cluster and kafka collection
schemamaker video_events.json -c my_cluster -k my_kafka

# All overrides, write to a specific directory
schemamaker video_events.json -n my_table -c my_cluster -k my_kafka -o migrations/
```

## Why this migration flow

Ingesting Kafka events into ClickHouse reliably requires three layers and two materialized views connecting them:

```
Kafka topic
    ↓
streams.{name}          # Kafka engine — ClickHouse reads raw messages from the topic
    ↓ (streams_mv)
raw.{name}              # stores the original message string + Kafka metadata (_key, _offset, _partition, _timestamp_ms, _topic)
    ↓ (raw_mv)
datalake.{name}         # typed, queryable table — each field JSONExtracted from the message
```

**Why keep `raw`?** Kafka messages are consumed once. `raw` acts as a durable replay buffer — if the schema changes or `datalake` needs to be rebuilt, you can re-run the materialized view against the stored messages without re-consuming from Kafka.

**Why `streams` as a separate table?** The Kafka engine table itself holds no data; it is just a cursor into the topic. Separating it from `raw` lets you pause, reset, or replace the consumer without touching stored data.

**Why `datalake` as the final table?** `raw` stores everything as `String` + metadata. `datalake` applies `JSONExtract` to give each field its proper type, making queries fast and ergonomic without re-parsing JSON at query time.

## Output

Two SQL files are written to the output directory:

**`{name}_up.sql`** — creates 5 objects in order:
1. `streams.{name}` — Kafka engine table (`message String`)
2. `raw.{name}` — ReplicatedMergeTree with Kafka metadata columns
3. `datalake.{name}` — ReplicatedMergeTree with inferred columns + metadata
4. `raw.{name}_mv` — materialized view that JSONExtracts each column from `raw` → `datalake`
5. `streams.{name}_mv` — materialized view that moves Kafka messages → `raw`

**`{name}_down.sql`** — drops all 5 objects in reverse dependency order.

## Type Inference

Types are inferred by scanning every record and widening as needed:

| JSON value | ClickHouse type |
|------------|-----------------|
| string | `Nullable(String)` |
| integer | `Nullable(Int64)` |
| float | `Nullable(Float64)` |
| boolean | `Nullable(Bool)` |
| null / array / object | `Nullable(String)` |

If the same field appears as `Int64` in one record and `Float64` in another, it widens to `Nullable(Float64)`. Any other type conflict widens to `Nullable(String)`. All columns are nullable unconditionally.

Field order in the output matches the first record that introduced each field.
