# Issue 013: Standard Library Primitives

## Status: Planning

## Goal
Expand the host function library beyond the 4 built-in functions shipped in 011a.
Coarse-grained capabilities (FsCap, NetCap) are sufficient for v0.1.

**Current host functions (shipped in 011a):**
- `read_file(fs: &FsCap, path: String) -> String & {Fs}`
- `write_file(fs: &FsCap, path: String, content: String) -> () & {Fs}`
- `now(time: &TimeCap) -> String & {Time}`
- `random_int(rand: &RandCap, min: Int, max: Int) -> Int & {Rand}`

**New host functions to add:**
- `http_get(net: &NetCap, url: String) -> String & {Net}`
- `http_post(net: &NetCap, url: String, body: String) -> String & {Net}`
- `file_exists(fs: &FsCap, path: String) -> Bool & {Fs}`
- `list_dir(fs: &FsCap, path: String) -> String & {Fs}` (newline-separated)
- `sleep(time: &TimeCap, ms: Int) -> () & {Time}`
- `parse_json(s: String) -> String` (pure — extract fields)
- `len(s: String) -> Int` (pure)
- `concat(a: String, b: String) -> String` (pure)
- `to_string(x: Int) -> String` (pure)
- `int_to_string(x: Int) -> String` (pure)

## Prerequisites
- Issue 011a (traced runtime — COMPLETE, v0.0.11)
- Issue 012 (affine integrity — runtime must be sound before adding more host functions)

## Notes
- All effectful host functions automatically participate in tracing (dispatch_traced)
- Pure host functions don't need capability params or tracing
- Network functions need real HTTP client (reqwest or ureq dependency)
- JSON parsing: return structured data or string extraction?
  Consider adding a simple path-based accessor: `json_get(json: String, path: String) -> String`

## Design Review: Not yet started
