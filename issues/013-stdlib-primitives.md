# Issue 013: Standard Library Primitives

## Status: Planning

## Goal
Implement the extern functions that make effects real:
- `read_file(path: String, fs: FsCap) -> String & {Fs}`
- `write_file(path: String, content: String, fs: FsCap) -> () & {Fs}`
- `http_get(url: String, net: NetCap) -> String & {Net}`
- `http_post(url: String, body: String, net: NetCap) -> String & {Net}`
- `now(time: TimeCap) -> Int & {Time}`
- `random(rand: RandCap) -> Int & {Rand}`

## Prerequisites
- Issue 011 (WASM runtime — these need host function bindings)
- Issue 012 (capability bundles — read vs write distinction)

## Design Review: Not yet started
