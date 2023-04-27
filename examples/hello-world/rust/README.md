# Hello world

This plugin is the bare-bones example.

You can copy it when creating your own plugins.

Configuration that your plugin exposes will automatically participate in `router.yaml`.

## Usage

```bash
cargo run -- --dev -s ../../graphql/supergraph.graphql -c ./router.yaml
```
cargo build --release --target x86_64-unknown-linux-musl 