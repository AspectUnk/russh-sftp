# Russh SFTP

SFTP subsystem supported server and client for [Russh](https://github.com/warp-tech/russh) and more!

Crate can provide compatibility with anything that can provide the raw data stream in and out of the subsystem channel.\
Implemented according to [version 3 specifications](https://datatracker.ietf.org/doc/html/draft-ietf-secsh-filexfer-02) (most popular).

The main idea of the project is to provide an implementation for interacting with the protocol at any level.

## Performance

Median throughput over six runs against OpenSSH in Docker (tmpfs), using the same 128 MiB payload and transfer settings: AES-128-GCM, 16 concurrent requests and `TCP_NODELAY=true`. Connection setup is excluded. Uploads wait for acknowledgements and close.

`TCP_NODELAY` disables Nagle's algorithm, reducing delays for small SFTP requests. Enable it with `russh::client::Config { nodelay: true, ..Default::default() }`. Go enables it by default.

| Implementation | Upload (MiB/s) | Download (MiB/s) |
| --- | ---: | ---: |
| russh-sftp + russh 0.63.2 | 257.3 | 203.1 |
| Node.js ssh2 1.17.0 | 253.3 | 197.9 |
| Go pkg/sftp 1.13.10 | 255.3 | 197.4 |

## Examples

- [Client example](https://github.com/AspectUnk/russh-sftp/blob/master/examples/client.rs)
- [Simple server](https://github.com/AspectUnk/russh-sftp/blob/master/examples/server.rs)

## What's ready?

- [x] Basic packets
- [x] Extended packets
- [x] Simplification for file attributes
- [x] Client side
- [x] Client example
- [x] Server side
- [x] Simple server example
- [ ] Full server example
- [x] Extension support: `limits@openssh.com`, `hardlink@openssh.com`, `fsync@openssh.com`, `statvfs@openssh.com`, `expand-path@openssh.com`
- [ ] Unit tests
- [x] Workflow

## Adopters

- [kty](https://github.com/grampelberg/kty) - The terminal for Kubernetes.

## Some words

Thanks to [@Eugeny](https://github.com/Eugeny) (author of the [Russh](https://github.com/warp-tech/russh)) for his prompt help and finalization of Russh API
