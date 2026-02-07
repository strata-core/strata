# Strata Examples

## Effect Trace Demo

`deploy.strata` demonstrates the full run-trace-replay workflow.

### Setup

```bash
mkdir -p /tmp/strata-demo
echo "database: postgres://localhost/myapp" > /tmp/strata-demo/config.txt
```

### Run with trace

```bash
cargo run -p strata-cli -- run examples/deploy.strata --trace /tmp/strata-demo/trace.jsonl
```

This produces a JSONL trace recording every host function call with
inputs, outputs, timing, and capability usage.

### View trace

```bash
cat /tmp/strata-demo/trace.jsonl | python3 -m json.tool
```

### Replay

```bash
cargo run -p strata-cli -- replay /tmp/strata-demo/trace.jsonl examples/deploy.strata
```

Replay verifies that the program's extern calls match the recorded trace,
substituting recorded outputs instead of calling real host functions.

### Replay-capable trace

Default `--trace` hashes large outputs (>1KB). For replay-capable traces:

```bash
cargo run -p strata-cli -- run examples/deploy.strata --trace-full /tmp/strata-demo/trace-full.jsonl
```

### Trace summary

```bash
cargo run -p strata-cli -- replay /tmp/strata-demo/trace.jsonl
```

Prints a summary of effects without replaying.
