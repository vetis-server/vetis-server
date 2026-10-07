# VeTiS (Very Tiny Server)

## Install

### Cargo

You can install it with `cargo install` or `cargo bininstall`:

```bash
cargo install vetis-server
```

## Quickstart

Here's how simple it is to create a web server with VeTiS:

```yaml
worker_threads: 24
max_blocking_threads: 120

server:
  workers: 1
  log:
    dest: "stdout"
    log_level: $(VETIS_LOG_LEVEL:"INFO")
  hosts:
    - hostname: "localhost"
      root_directory: "sites/default"
      enable_hsts: false
      allow_unsafe_conn: false
      bind_addresses:
        - ["0.0.0.0", 443]
      protos:
        - "HTTP/1.1"
        - "HTTP/2.0"
      security:
        ca_cert_from_file: "certs/ca.der"
        cert_from_file: "certs/server.der"
        key_from_file: "certs/server.key.der"
      error_pages:
        404: "404.html"
      paths:
        - type: "static"
          uri: "/"
          directory: "html"
          extensions: "\\.(html)$"
          index_files:
            - "index.html"
        - type: "flash_path"
          uri: "/vetis"
          response: "Hello from VeTiS!"
          status_code: 200
        - type: "flash_path"
          uri: "/vetis/rio"
          response: "Rust rocks!"
          status_code: 200
```

Then you can run with:

```bash
vetis-server --f vetis.yaml
```

Please note this sample configuration is also available at repository root.

## License

Licensed under either of

- Apache License, Version 2.0
  (LICENSE-APACHE or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  (LICENSE-MIT or <https://opensource.org/licenses/MIT>)

at your option.

## Author

Rogerio Pereira Araujo <rogerio.araujo@gmail.com>
