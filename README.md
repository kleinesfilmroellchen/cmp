# CMP - the Camping Madness Project

This is the repository for the “Camping Madness Project”, a pixely micro-management sim game where you run your own campsite.

CMP is in very early prototyping development and I don’t know where it will go entirely!

## Usage

CMP is a regular cargo-capable Rust project that runs on the experimental Bevy game engine. It should work on all supported Bevy platforms, though desktop input methods (mouse & keyboard) are the primary target.

Compiling CMP might take a while initially, since the engine is compiled with optimizations and linked into a separate dynamic library. A nightly compiler is required. The last tested nightly compiler is 2024-07-29.

Since most game settings cannot be changed within the game itself, it is recommended to use a local configuration file that can be edited manually. For example:

```
cargo run -- --settings-file debugconf.toml
```
