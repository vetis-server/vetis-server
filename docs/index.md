---
layout: default
title: VeTiS - Very Tiny Server
nav_order: 1
description: "A blazingly fast, minimalist HTTP server built for modern Rust applications"
permalink: /
---
<div align="center">
<h1><b>VeTiS</b></h1>
</div>

This project started inside VeTiS, the reference API nowadays. Then it moved to vetis-tokio
and related runtime crates, but now, for better packaging it has his own repository.

## Features

- **Minimalist Design**: Focus on what matters - serving HTTP requests efficiently
- **Protocol Support**: Full HTTP/1, HTTP/2, and HTTP/3 implementation
- **Secure by Default**: Built-in TLS support with modern cryptography
- **Zero-Cost Abstractions**: Leverage Rust's performance without overhead
- **Language Support**: Built-in support for Python, PHP, and RSGI applications
- **Feature-Gated**: Include only what you need for optimal binary size

## Quickstart

Here's how simple it is to create a web server with VeTiS:

```yaml
worker_threads: 24
max_blocking_threads: 120

server:
  enable_logging: true
  listeners:
    - interface: "0.0.0.0"
      port: 443
      protos:
        - "HTTP/1.1"
        - "HTTP/2.0"
      alpn_protos:
        - "h2"
        - "http/1.1"
      allow_unsafe_conn: false
  hosts:
    - hostname: "localhost"
      root_directory: "sites/default"
      enable_hsts: false
      bind_addresses:
        - ["0.0.0.0", 8443]
      security:
        ca_cert_from_file: "certs/ca.der"
        cert_from_file: "certs/server.der"
        key_from_file: "certs/server.key.der"
      error_pages:
        404: "404.html"
      paths:
        - type: "static_path"
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

## Documentation

- [Contributing Guide](./CONTRIBUTING.md)
- [Language Support](./LANGUAGE_SUPPORT.md)

## Other Projects

- [caramelo](https://crates.io/crates/caramelo) - Assertion based test framrwork
- [deboa](https://crates.io/crates/deboa) - HTTP client
- [easyhttpmock](https://crates.io/crates/easyhttpmock) - HTTP mock server
- [sofie](https://crates.io/crates/sofie) - Fullstack web framework
- [uget](https://crates.io/crates/uget) - CLI HTTP client

## License

Licensed under either of

- Apache License, Version 2.0
  (LICENSE-APACHE or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  (LICENSE-MIT or <https://opensource.org/licenses/MIT>)

at your option.

## Author

Rogerio Pereira Araujo <rogerio.araujo@gmail.com>
