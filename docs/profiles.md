# Profiles

## Kernel
- No `Net`, `FS`, `Time`, `Rand`; no dynamic alloc; deterministic only.

## Realtime
- Bounded latency; backpressure; no GC; limited `Net`/`FS`; seeded `Rand`.

## General (default)
- Full language within capability model.
