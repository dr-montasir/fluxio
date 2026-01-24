### v0.5.2

- Updated Modules:
    - bytes (v1.0 => v1.11.0).
    - tokio (v1 => v1.49.0)
    - h2 (v0.3.9 => v0.4.13)
    - itoa (v1 => v1.0.17)
    - httparse (v1.6 => v1.10.1)
    - futures-core (v0.3 => v0.3.31)
    - futures-channel (v0.3 => v0.3.31)
    - futures-util (v0.3 => v0.3.31)
    - pretty_env_logger (v0.4 => v0.5.0)
    - socket2 (v0.4 => v0.6.2)
    - http (v0.2 => v0.2.12) * 
        - (Errors when updating to the latest version 1.4.0) 
            - (src/body/body.rs => line 373)
    - http-body (v0.4 => v0.4.6) * 
        - (Errors when updating to the latest version 1.0.1)
            - (src/body/body.rs => lines 359 - 398 => impl HttpBody for Body)
            - (src/body/aggregate.rs => line 23 => no method named `data` found ..)
            - (src/body/to_bytes.rs => line 72 => no method named `data` found ..)
    - tower (v0.4 => v0.5.3)
    - pnet_datalink (v0.27.2 => v0.35.0)

### v0.5.1

- Updated Modules:
    - url (v2.2 => v2.5.8).

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
