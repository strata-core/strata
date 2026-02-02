# Effects & Capabilities

- Effects are explicit on function types: `fn f(A) -> B & { Net, Time }`.
- Capabilities gate access to effects: `NetCap`, `FsCap`, `TimeCap`, `RandCap`.
- No ambient I/O: a function must both *declare* the effect and *receive* the capability.
- **Layered Context** sugar:
```strata
layer Analytics(using net: NetCap, fs: FsCap) & { Net, FS } {
    let body = http_get(Url("..."), using net)?
    write_file("/tmp/out.txt", body, using fs)
}
```
