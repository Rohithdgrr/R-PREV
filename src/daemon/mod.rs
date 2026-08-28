//! Daemon — Unix socket / Named Pipe pre-warm for <5ms hot startup
//! Client checks daemon.sock; if live sends CBOR Open {path}, daemon forks TUI session sharing warm Cache/SyntaxSet/WasmStore.
