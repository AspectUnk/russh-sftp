# Russh SFTP
Crate for [Russh](https://github.com/warp-tech/russh) to support the SFTP server and client subsystem.\
Implemented according to [version 3 specifications](https://filezilla-project.org/specs/draft-ietf-secsh-filexfer-02.txt) (most popular)

## Examples
- [Simple server](https://github.com/AspectUnk/russh-sftp/blob/master/examples/server.rs)
- ~~Fully implemented server~~
- ~~Client example~~

## What's ready?
- [x] Basic packets
- [x] Extended packets
- [x] Simplification for file attributes
- [x] Server side
- [x] Simple server example
- [ ] Error handler (unlike specification)
- [ ] Checking for compliance specification
- [ ] Full server example
- [ ] Unit tests
- [ ] Workflow
- [ ] Client side
- [ ] Client example

## Some words
Thanks to [@Eugeny](https://github.com/Eugeny) (author of the [Russh](https://github.com/warp-tech/russh)) for his prompt help and finalization of Russh API
