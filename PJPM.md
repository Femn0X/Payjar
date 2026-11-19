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
- `pjpm search <query>` or `search <name>@<constraint>` — discover packages and versions (for example `Geometry@1.0`).
- `pjpm info <name>[@constraint]`, `list` — inspect a specific release or list installed packages.
- `pjpm build [--approve-build]` — run the declared build adapter after confirmation.
- `pjpm run [args...]` — run the declared runtime entry point.
- `pjpm doctor` — show available language toolchains.
- `pjpm pack` — create a source archive and print its SHA-256 digest.

## Adding dependencies to a project

`pjpm add` records a dependency in the **current project's** `pjpackage.toml`. Running it from the Payjar compiler repository without a package manifest is not supported. Create or open a separate project first:

```sh
mkdir -p ~/payjar-demo
cd ~/payjar-demo
python3 ~/Payjar/pjpm.py init payjar-demo
python3 ~/Payjar/pjpm.py search Geometry@1.0
python3 ~/Payjar/pjpm.py add Geometry@1.0
python3 ~/Payjar/pjpm.py list
```

`search` can filter available versions by SemVer constraint; `info Geometry@1.0` displays that particular release. Legacy two-component versions like `1.0` remain supported as exact version strings.

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
# Geometry = "1.0"
```

Adapters available out of the box: Python, Node.js, Payjar (`PJC`), Rust/Cargo, Go, Java/Gradle, and CMake. A package can use any language by declaring `adapter = "custom"` with a `command = ["program", "arg"]` array. Commands are argument vectors, never shell strings. Install does not execute package build scripts.

## Versioned registry and compatibility (pjpm 0.3)

The default registry is `https://github.com/Femn0X/Payjar-Packages`; set `PJPM_REGISTRY` to a different GitHub repository URL to override it.

The registry may publish a root `releases.json` index (schema 1) containing any number of package versions. Each entry has a **full immutable Git commit SHA** and a package path within that commit. `pjpm` reads and validates each pinned package manifest, selects a compatible version, resolves its transitive dependencies, and downloads the source from that fixed commit. The dependency resolver can backtrack when the newest release causes a constraint conflict. New v2 releases use SemVer; legacy releases such as `Geometry@1.0` keep their original version strings.

When a package does not have an index entry, `pjpm` falls back to its original root folder on the registry's `main` branch, supporting existing `pjpackage.toml`, `package.toml`, and `init.pj` layouts. Such **unindexed legacy sources are mutable**, unlike indexed releases.

```sh
pjpm add Geometry@1.0
pjpm install
pjpm info Geometry
pjpm update              # explicitly select newer compatible releases
```

`pjpm install` prefers versions and source commits already recorded in `pjpm.lock`. `pjpm update` selects the newest compatible releases instead. The lockfile stores the exact version, registry, source commit/path when indexed, and the SHA-256 digest of the downloaded source archive. Install verifies the saved digest, even when a copy of the package is already present. Cached packages live under `.pjpm/packages/<name>/<version>`; extraction is staged before replacing an installed version, and a `.pjpm-source.json` file records which archive produced it.

Supported constraints include exact versions, `^`, `~`, `>=`, `<=`, `>`, `<`, `!=`, and comma-separated ranges. `pjpm search` and `pjpm info` can discover indexed releases and root-format packages.

## Security and limitations

- Installing copies package source and **does not run package build scripts**. Builds require explicit approval (`--approve-build` is an explicit opt-in).
- Manifest commands use argument arrays rather than shell strings; **builds and runs are not sandboxed**.
- Source paths and ZIP extraction paths are validated. The installer rejects archives exceeding 20,000 entries or 512 MiB of declared uncompressed data. It still downloads the whole repository archive for each source commit; dedicated package archives are future work.
- Release entries must point to full commit SHAs, but release metadata is **not cryptographically signed** and is not currently audited against previous registry entries. GitHub repository permissions and review policies remain important.
- Publishing automation, signed provenance, a package-level artifact registry, a cross-language ABI, and WASI/native execution bridges are not implemented yet. Authoring packages in different languages does not imply transparent imports between those languages.

Run the unit tests with `python -m unittest discover -s tests -v`.
