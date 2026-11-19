# pjpm — Payjar Package Manager

`pjpm` is included in this repository as a Python 3.11+ CLI. It uses only the Python standard library; no pip install is required.

## Install / run

From a checkout of Payjar:

```sh
chmod +x pjpm
./pjpm --help
```

Or run directly with `python3 pjpm.py`. To make it available globally, copy `pjpm` and `pjpm.py` into a directory on your PATH. Windows users can run `py -3 pjpm.py`.

## Commands

- `pjpm init [name]` — create a v2 manifest in the current directory.
- `pjpm create <name>` — create a project directory and manifest.
- `pjpm add <name>[@constraint]` — add and install a registry dependency.
- `pjpm install` — install manifest dependencies and update `pjpm.lock`.
- `pjpm remove <name>` / `uninstall <name>` — remove a dependency.
- `pjpm search <query>`, `info <name>`, `list` — discover and inspect packages.
- `pjpm build [--approve-build]` — run the declared build adapter after confirmation.
- `pjpm run [args...]` — run the declared runtime entry point.
- `pjpm doctor` — show available language toolchains.
- `pjpm pack` — create a source archive and print its SHA-256 digest.

## Universal manifest

```toml
[package]
schema = 2
name = "hello-python"
version = "1.0.0"
description = "Example"
kind = "executable"
language = "python"

[build]
adapter = "python"
args = ["-m", "compileall", "src"]

[runtime]
adapter = "python"
entry = "src/main.py"

[dependencies]
# geometry = "^1.0.0"
```

Adapters available out of the box: Python, Node.js, Payjar (`PJC`), Rust/Cargo, Go, Java/Gradle, and CMake. A package can use any language by declaring `adapter = "custom"` with a `command = ["program", "arg"]` array. Commands are argument vectors, never shell strings. Install does not execute package build scripts.

## Registry and compatibility

The default registry is `https://github.com/Femn0X/Payjar-Packages`; override it with `PJPM_REGISTRY` pointing to another GitHub repository. Current registry layout is one folder per package. The client supports v2 `pjpackage.toml`, legacy `package.toml`, and legacy `init.pj` packages when inspecting local projects. Registry packages are fetched from the registry's `main` branch; version constraints are checked against its published manifest version. The client resolves transitive dependencies, detects cycles and conflicting constraints, and supports basic SemVer comparators (`^`, `~`, `>=`, `<=`, `>`, `<`, `!=`, exact versions, and comma-separated ranges). The current folder registry does not provide multiple historical versions, so constraints are checked against each package's current manifest version; it cannot select an older matching release.

Packages install under `.pjpm/packages/<name>/<version>`. `pjpm.lock` records exact versions and archive SHA-256 digests. Locked installs re-fetch the archive and verify the digest. This initial client does not claim transparent cross-language imports: use `pjpm run` or an explicit process boundary. Native/wasm ABI loading is not enabled.

## Security and limitations

- Builds require explicit approval; `--approve-build` is an explicit opt-in.
- Manifest commands are arrays; no shell interpolation or shell parsing occurs.
- Registry ZIP paths are checked against traversal before extraction.
- Installing a package copies source files; it does not execute them.
- Registry publishing, signed metadata, transitive dependency resolution, sandboxed builds, and WASI/native ABI bridges are not implemented in this first working release.
