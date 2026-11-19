# Payjar Universal Package System v2 — Design and Implementation Plan

Status: design baseline. This document does not claim v2 commands or adapters are implemented yet.

## Goals

Payjar packages can contain source in any language for which the host has a supported toolchain or portable artifact. The package manager owns identity, versions, dependency resolution, integrity, installation layout, and adapter selection. It must not pretend arbitrary source can be imported directly into the Payjar interpreter.

## Repository findings

- The interpreter and CLI are Rust-based; the language runtime is Payjar.
- The registry currently documents a required `init.pj` entry point.
- Existing registry `package.toml` files use legacy top-level keys: `description`, `language`, `build`, and `version`.
- Registry documentation also mentions `pjpackage.toml`, creating a manifest-name mismatch.
- The package-manager implementation was not present in the tracked Payjar source tree reviewed. Locate/consolidate the distributed `pjpm` client before wiring this format into commands.

## Canonical v2 manifest

Filename: `pjpackage.toml`.

```toml
[package]
schema = 2
name = "math-tools"
version = "1.0.0"
description = "Small reusable math utilities"
kind = "library"
language = "python"

[build]
adapter = "python"
args = ["-m", "build"]

[runtime]
adapter = "python"
entry = "src/math_tools.py"

[dependencies]
geometry = "^1.0.0"
```

Rules:
1. `package.name` is the registry identity and matches the lowercase package slug.
2. `package.version` is SemVer.
3. `package.kind` is `library`, `executable`, `plugin`, `native`, `wasm`, or `source`.
4. `package.language` identifies the primary language adapter, not a restriction on package contents.
5. `build.adapter` and `runtime.adapter` select registered adapters; they are not shell snippets.
6. `args` is an argument array. Never concatenate it into a shell command.
7. Registry dependencies use SemVer constraints. External ecosystem dependencies remain the adapter's responsibility and should use that ecosystem's lockfile.
8. Unknown required schema versions fail with an actionable error.

A package may contain multiple languages. Do not infer a language by executing package files.

## Package kinds and interoperability

| Kind | Meaning |
| --- | --- |
| `library` | Reusable package for its declared runtime |
| `executable` | Program launched as a process |
| `plugin` | Payjar extension loaded through a documented plugin ABI |
| `native` | Compiled library with an explicit platform/ABI |
| `wasm` | WebAssembly module with a declared WASI/component interface |
| `source` | Source distribution requiring a matching adapter/toolchain |

Cross-language calls require an explicit boundary: JSON messages over stdin/stdout, WASI/WebAssembly components, or a native library with an explicit target and ABI. A Python module is not automatically importable as a Payjar module. Transparent imports must wait until the bridge and type mapping are implemented and tested.

## Backward compatibility

The v2 reader must use this precedence:
1. `pjpackage.toml` with `[package].schema = 2`.
2. `package.toml` parsed using the legacy top-level schema.
3. `init.pj` without either manifest as a legacy Payjar package.

Compatibility requirements:
- Existing folders containing `init.pj` remain installable.
- Existing `package.toml` keys keep their current meanings.
- Do not rename/delete legacy files during migration.
- Legacy build commands require explicit trust/approval; never silently execute them during install.
- If both manifests exist, new clients use v2 and old clients retain their legacy behavior.
- Migration tooling reports changes and never destructively rewrites packages.

## Adapter contract

Each registered adapter implements:
- `detect(manifest, host)`: report toolchain/platform availability without executing package code.
- `validate(manifest)`: validate adapter-specific configuration.
- `build(package_dir, plan, sandbox)`: execute an argument-vector plan in an isolated workspace.
- `install(package_dir, install_dir, artifacts)`: install declared outputs without escaping the install root.
- `run(package_dir, plan, request)`: optional runtime bridge.

Return structured errors, tool versions, artifact paths, and exit status. Never accept arbitrary shell strings from a manifest. Initial adapters: Payjar, Python, Node.js, Rust/Cargo, Go, Java, and C/C++.

## Dependencies and lockfile

- Resolve registry dependencies by package name and SemVer.
- Detect cycles and incompatible constraints before modifying the project.
- Write deterministic `pjpm.lock` with exact versions, registry source, manifest digest, and SHA-256 archive digest.
- Locked installs do not silently upgrade.
- Separate Payjar registry dependencies from adapter-managed dependencies.
- Reject path traversal, duplicate identities, and archive entries escaping the install root.

## Security

- Build scripts are opt-in with a clear approval prompt.
- Adapters run with least privilege, a clean environment, bounded output/time/resources, and a temporary workspace.
- Validate archive paths and symlinks before extraction.
- Verify SHA-256 digests; add signed registry metadata in a later phase.
- Never load native libraries/plugins merely because they were downloaded.
- Show the toolchain and build action before approval.

## CLI target

```text
pjpm init
pjpm create <name>
pjpm add <name> [version]
pjpm install
pjpm remove <name>
pjpm update [name]
pjpm search <query>
pjpm info <name>
pjpm build [--approve-build]
pjpm publish
pjpm list
pjpm doctor
```

Keep existing aliases such as `install`, `uninstall`, and `upload` until a deprecation cycle is complete.

## Delivery plan

### Phase 0 — establish the client
- Locate the distributed `pjpm` source/binary and release process.
- Capture current legacy install/list/search/upload behavior in integration tests.
- Document the actual current CLI syntax and package layout.

### Phase 1 — manifest and compatibility
- Add a TOML parser and typed v2 manifest model to the actual package-manager crate.
- Implement manifest precedence and schema validation.
- Add fixtures for v2, legacy `package.toml`, and `init.pj`-only packages.

### Phase 2 — registry and resolver
- Add version discovery, SemVer resolution, archive digests, and `pjpm.lock`.
- Preserve the folder-based registry while introducing versioned metadata.

### Phase 3 — adapters and safe builds
- Implement Payjar and Python adapters first, then Node, Rust, Go, Java, and C/C++.
- Use argument arrays, approval prompts, isolated workspaces, and structured build results.
- Add toolchain detection and `pjpm doctor`.

### Phase 4 — interoperability
- Define the JSON process protocol and value mapping.
- Add WASI/component support where runtime dependencies permit.
- Add native ABI support only with explicit target/ABI declarations.
- Integrate adapter calls into Payjar imports only after end-to-end tests pass.

### Phase 5 — publishing and ecosystem
- Add registry validation and reproducible build checks.
- Add signed metadata, provenance, vulnerability reporting, and deprecation policy.
- Publish adapter authoring documentation.

## Acceptance criteria

- Current legacy packages install unchanged.
- A v2 Python package validates/builds/installs without `init.pj` when Python tooling is present.
- Existing Payjar packages keep importing through the current mechanism.
- Missing toolchains produce actionable diagnostics.
- Builds are opt-in and isolated; manifest strings are never shell-parsed.
- Lockfile installs are deterministic and verify archive digests.
- Unsupported cross-language combinations fail clearly rather than pretending to import.
