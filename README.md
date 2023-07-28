# Snippets LS
## _Simple snippets language server_

Snippets LS is simple, toml configurable, Language Server.

## Tech

Snippets LS uses a number of open source projects to work properly:

- [[lsp-server]](https://crates.io/crates/lsp-server) - Generic LSP server scaffold.
- [[lsp-types]](https://crates.io/crates/lsp-types) - Types for interaction with a language server, using VSCode's Language Server Protocol

## Installation

Snippets LS requires [Rust](https://www.rust-lang.org/) to compile.

```sh
git clone git@github.com:Stanislav-Lapata/snippets-ls.git
cd snippets-ls
cargo install --path .
```

## Development

Want to contribute? Great!