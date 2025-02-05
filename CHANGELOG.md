### v0.5.0

- Included Kind::Parse(Parse::Internal) in src/error.rs.
- Yanked versions 0.0.0, 0.1.0, 0.3.0, and 0.4.0.

### v0.4.0

- `cargo clippy -- -D warnings` (test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s)
- Renamed the module from `payload` back to `body` (src/body/payload.rs => src/body/body.rs)

### v0.3.0

- `cargo clippy -- -D warnings` (test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.62s)
- Renamed the module from `body` to `payload` (src/body/body.rs => src/body/payload.rs)

### v0.2.0

- Used `futures_util::never::Never`
- Used `crate::header::HeaderName`

### v0.1.0

- Init
