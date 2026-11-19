# Config-based pjpm installation

This example declares `Geometry@1.0.1` in `config.toml` and imports its Payjar `init.pj`.

From the Payjar checkout (after merging the runtime PR and the [Geometry 1.0.1 release](https://github.com/Femn0X/Payjar-Packages/pull/3)):

```sh
cargo run -- examples/config-packages/main.pj
```

The runtime will invoke `pjpm sync` in this project's directory, download the dependency once, and record its resolved source in `examples/config-packages/pjpm.lock`. Later runs reuse installed dependencies if the lock and cache match.

The expected output from the example's function is approximately `5`.

**Requirements:** Python 3.11+, GitHub registry access on the first run, and a compiled Payjar interpreter. No cross-language transpilation is required for Payjar-native packages. Non-Payjar sources are installed but are not directly importable without an explicit bridge.
