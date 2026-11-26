#!/usr/bin/env python3
"""Payjar package manager (pjpm). Requires Python 3.11+; standard library only."""
from __future__ import annotations
import argparse, functools, hashlib, io, json, os, re, shutil, subprocess, sys, tempfile, tomllib, urllib.error, urllib.parse, urllib.request, zipfile
from pathlib import Path, PurePosixPath

VERSION = "0.3.0"
REGISTRY = os.environ.get("PJPM_REGISTRY", "https://github.com/Femn0X/Payjar-Packages")
API = "https://api.github.com"
LOCK = "pjpm.lock"
MANIFEST = "pjpackage.toml"
LEGACY = "package.toml"

class PjpmError(Exception): pass

def http(url, binary=False, missing_ok=False):
    req = urllib.request.Request(url, headers={"User-Agent":"pjpm/0.3", "Accept":"application/vnd.github+json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as res:
            return res.read() if binary else res.read().decode("utf-8")
    except urllib.error.HTTPError as e:
        if missing_ok and e.code == 404:
            return None
        raise PjpmError(f"request failed: {url}: {e}") from e
    except Exception as e:
        raise PjpmError(f"request failed: {url}: {e}") from e

def safe_name(name):
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]*", name) or name in (".", ".."):
        raise PjpmError(f"invalid package name: {name!r}")
    return name

def safe_version(version):
    if not isinstance(version, str) or not re.fullmatch(r"[0-9A-Za-z][0-9A-Za-z._+-]*", version):
        raise PjpmError(f"invalid package version: {version!r}")
    return version


def load_manifest(root=Path(".")):
    v2 = root / MANIFEST
    old = root / LEGACY
    if v2.exists():
        try: data = tomllib.loads(v2.read_text(encoding="utf-8"))
        except Exception as e: raise PjpmError(f"invalid {MANIFEST}: {e}")
        p = data.get("package", {})
        if p.get("schema") != 2: raise PjpmError("pjpackage.toml must declare [package].schema = 2")
        for key in ("name","version","language"):
            if not isinstance(p.get(key), str) or not p[key]: raise PjpmError(f"missing [package].{key}")
        safe_name(p["name"])
        if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", p["version"]):
            raise PjpmError("package.version must be SemVer (for example 1.2.3)")
        p["kind"] = p.get("kind", "library")
        return {"format":"v2", "name":p["name"], "version":p["version"], "language":p["language"].lower(),
                "description":p.get("description",""), "kind":p["kind"], "build":data.get("build",{}),
                "runtime":data.get("runtime",{}), "dependencies":data.get("dependencies",{}), "raw":data}
    if old.exists():
        try: d = tomllib.loads(old.read_text(encoding="utf-8"))
        except Exception as e: raise PjpmError(f"invalid legacy package.toml: {e}")
        name = root.resolve().name
        return {"format":"legacy", "name":name, "version":safe_version(str(d.get("version","0.0.0"))),
                "language":str(d.get("language","payjar")).lower(), "description":str(d.get("description","")),
                "kind":"library", "build":{"adapter":str(d.get("language","payjar")).lower(),"legacy_build":d.get("build")},
                "runtime":{"adapter":str(d.get("language","payjar")).lower(),"entry":"init.pj"},
                "dependencies":{}, "raw":d}
    if (root / "init.pj").exists():
        return {"format":"legacy-init", "name":root.resolve().name, "version":"0.0.0", "language":"payjar",
                "description":"", "kind":"library", "build":{}, "runtime":{"adapter":"payjar","entry":"init.pj"},
                "dependencies":{}, "raw":{}}
    raise PjpmError(f"no {MANIFEST}, {LEGACY}, or init.pj found in {root}")

def registry_repo():
    u = urllib.parse.urlparse(REGISTRY)
    if u.netloc != "github.com": raise PjpmError("PJPM_REGISTRY must be a GitHub repository URL")
    return u.path.strip("/").removesuffix(".git")

def registry_dirs():
    repo = registry_repo()
    data = json.loads(http(f"{API}/repos/{repo}/git/trees/HEAD?recursive=1"))
    roots = set()
    for item in data.get("tree", []):
        parts = item.get("path", "").split("/")
        if len(parts) == 2 and item.get("type") == "blob" and parts[1] in (MANIFEST, LEGACY, "init.pj"):
            roots.add(parts[0])
    return sorted(roots | set(registry_index()))


def safe_registry_path(path):
    if not isinstance(path, str) or not path or "\\" in path:
        raise PjpmError(f"invalid registry source path: {path!r}")
    p = PurePosixPath(path)
    if p.is_absolute() or any(x in (".", "..", "") for x in path.split("/")):
        raise PjpmError(f"unsafe registry source path: {path!r}")
    return path


@functools.lru_cache(maxsize=1)
def registry_index():
    repo = registry_repo()
    raw = http(f"https://raw.githubusercontent.com/{repo}/main/releases.json", missing_ok=True)
    if raw is None:
        return {}
    try:
        doc = json.loads(raw)
        if not isinstance(doc, dict) or doc.get("schema") != 1 or not isinstance(doc.get("packages"), dict):
            raise ValueError("expected schema 1 and packages object")
        for name, versions in doc["packages"].items():
            safe_name(name)
            if not isinstance(versions, dict) or not versions:
                raise ValueError(f"{name}: versions must be a nonempty object")
            for version, entry in versions.items():
                if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+(?:\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)?", version) or not isinstance(entry, dict):
                    raise ValueError(f"{name}: invalid release record")
                commit = entry.get("commit")
                if not isinstance(commit, str) or not re.fullmatch(r"[0-9a-f]{40}", commit):
                    raise ValueError(f"{name}@{version}: release must use a full immutable commit SHA")
                safe_registry_path(entry.get("path"))
        return doc["packages"]
    except (ValueError, TypeError, PjpmError) as e:
        raise PjpmError(f"invalid registry releases.json: {e}") from e


def read_registry_manifest(name, ref, path):
    safe_name(name)
    safe_registry_path(path)
    if ref != "main" and not re.fullmatch(r"[0-9a-f]{40}", ref):
        raise PjpmError("package source must be the legacy main branch or an immutable commit SHA")
    repo = registry_repo()
    base = f"https://raw.githubusercontent.com/{repo}/{ref}/{urllib.parse.quote(path, safe='/')}"
    for filename in (MANIFEST, LEGACY):
        raw = http(f"{base}/{filename}", missing_ok=True)
        if raw is None:
            continue
        try:
            data = tomllib.loads(raw)
        except (ValueError, TypeError) as e:
            raise PjpmError(f"{name}: invalid {filename} at {ref}: {e}") from e
        if filename == MANIFEST:
            pkg = data.get("package", {})
            if pkg.get("schema") != 2 or pkg.get("name") != name:
                raise PjpmError(f"{name}: v2 manifest schema or name mismatch at {ref}")
            version = pkg.get("version")
            if not isinstance(version, str) or not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?", version):
                raise PjpmError(f"{name}: v2 release version must be SemVer")
            deps = data.get("dependencies", {})
            language = pkg.get("language", "unknown")
        else:
            version = safe_version(str(data.get("version", "0.0.0")))
            deps = {}
            language = data.get("language", "payjar")
        if not isinstance(deps, dict) or any(not isinstance(x, str) for x in deps.values()):
            raise PjpmError(f"{name}: dependencies must map package names to constraint strings")
        return {"name": name, "version": version, "language": language,
                "manifest": filename, "dependencies": deps,
                "commit": None if ref == "main" else ref, "path": path}
    if http(f"{base}/init.pj", missing_ok=True) is not None:
        return {"name": name, "version": "0.0.0", "language": "payjar",
                "manifest": "init.pj", "dependencies": {},
                "commit": None if ref == "main" else ref, "path": path}
    raise PjpmError(f"package {name!r} has no manifest or init.pj at {ref}:{path}")


def version_sort_key(version):
    # Legacy package.toml may use major.minor rather than major.minor.patch.
    match = re.fullmatch(r"(\d+)\.(\d+)(?:\.(\d+))?(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?", version)
    if not match:
        return (-1, -1, -1, 0, ())
    pre = match.group(4)
    identifiers = tuple((0, int(p)) if p.isdigit() else (1, p) for p in pre.split(".")) if pre else ()
    return (int(match[1]), int(match[2]), int(match[3] or 0), 0 if pre else 1, identifiers)


def registry_candidates(name):
    safe_name(name)
    entries = registry_index().get(name)
    if entries is None:
        return [read_registry_manifest(name, "main", name)]
    releases = []
    for version, entry in entries.items():
        meta = read_registry_manifest(name, entry["commit"], entry["path"])
        if meta["version"] != version:
            raise PjpmError(f"{name}@{version}: index version differs from pinned manifest version {meta['version']}")
        releases.append(meta)
    return sorted(releases, key=lambda meta: version_sort_key(meta["version"]), reverse=True)


def registry_manifest(name, constraint="*"):
    candidates = registry_candidates(name)
    for meta in candidates:
        if version_ok(meta["version"], constraint):
            return meta
    raise PjpmError(f"{name}: no version satisfies {constraint!r}; available: {', '.join(x['version'] for x in candidates)}")

def parse_spec(spec):
    if "@" in spec:
        name, constraint=spec.rsplit("@",1)
        return safe_name(name), constraint or "*"
    return safe_name(spec), "*"

def version_ok(version, constraint):
    if constraint in ("", "*", "latest"):
        return True
    value = version_sort_key(version)
    for part in (x.strip() for x in constraint.split(",")):
        if not part or part in ("*", "latest"):
            continue
        op = next((x for x in (">=", "<=", "!=", ">", "<", "^", "~", "=") if part.startswith(x)), "")
        raw = part[len(op):].strip()
        bound = version_sort_key(raw)
        if op in ("", "="):
            ok = version == raw
        elif op == "!=":
            ok = version != raw
        elif value[0] < 0 or bound[0] < 0:
            ok = False
        elif op == "^":
            major, minor, patch = bound[:3]
            if major > 0:
                upper = (major + 1, 0, 0)
            elif minor > 0:
                upper = (0, minor + 1, 0)
            else:
                upper = (0, 0, patch + 1)
            ok = value >= bound and value[:3] < upper
        elif op == "~":
            ok = value >= bound and value[:2] == bound[:2]
        elif op == ">=":
            ok = value >= bound
        elif op == "<=":
            ok = value <= bound
        elif op == ">":
            ok = value > bound
        else:
            ok = value < bound
        if not ok:
            return False
    return True

def project_dependencies():
    """Merge project v2 dependencies with config.toml [dependencies]."""
    dependencies = {}
    if Path(MANIFEST).is_file() or Path(LEGACY).is_file() or Path("init.pj").is_file():
        dependencies.update(load_manifest()["dependencies"])
    config = Path("config.toml")
    if config.is_file():
        try:
            data = tomllib.loads(config.read_text(encoding="utf-8"))
        except (ValueError, OSError) as e:
            raise PjpmError(f"invalid config.toml: {e}") from e
        declared = data.get("dependencies", {})
        if not isinstance(declared, dict):
            raise PjpmError("config.toml [dependencies] must be a TOML table")
        for name, constraint in declared.items():
            safe_name(name)
            if not isinstance(constraint, str) or not constraint.strip():
                raise PjpmError(f"config.toml: {name} must have a nonempty version constraint string")
            if name in dependencies and dependencies[name] != constraint:
                constraint = f"{dependencies[name]},{constraint}"
            dependencies[name] = constraint
    if not dependencies and not (Path(MANIFEST).exists() or Path(LEGACY).exists()
                                 or Path("init.pj").exists() or config.is_file()):
        raise PjpmError("no project manifest or config.toml found in the current directory")
    return dependencies


def remote_dependencies(dependencies):
    """Existing local Payjar package sources take precedence over registry entries."""
    return {name: requirement for name, requirement in dependencies.items()
            if not (Path(name) / "init.pj").is_file()}


def read_lock():
    path = Path(LOCK)
    if not path.is_file():
        return {}
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (ValueError, OSError) as e:
        raise PjpmError(f"invalid {LOCK}: {e}") from e
    if not isinstance(value, dict) or not isinstance(value.get("packages", []), list):
        raise PjpmError(f"invalid {LOCK}: expected object with packages list")
    return value


def packages_ready(requirements):
    """No network is needed if the exact dependency set and cached packages match the lock."""
    lock = read_lock()
    if lock.get("project_dependencies") != requirements:
        return False
    records = lock.get("packages", [])
    pinned = {item["name"]: item for item in records if isinstance(item, dict) and "name" in item}
    if any(name not in pinned or not version_ok(pinned[name].get("version", ""), constraint)
           for name, constraint in requirements.items()):
        return False
    for item in records:
        if not isinstance(item, dict):
            return False
        try:
            name = safe_name(item["name"])
            version = safe_version(item["version"])
        except (KeyError, PjpmError, TypeError):
            return False
        source_file = Path(".pjpm/packages") / name / version / ".pjpm-source.json"
        if not source_file.is_file():
            return False
        try:
            if json.loads(source_file.read_text(encoding="utf-8")) != item:
                return False
        except (OSError, ValueError):
            return False
    return True


def add_dependency(name, constraint):
    path=Path(MANIFEST)
    if not path.exists(): raise PjpmError("pjpm add requires a v2 pjpackage.toml; run pjpm init first")
    text=path.read_text()
    if "[dependencies]" not in text: text=text.rstrip()+"\n\n[dependencies]\n"
    lines=text.splitlines()
    start=next(i for i,x in enumerate(lines) if x.strip()=="[dependencies]")
    end=next((i for i in range(start+1,len(lines)) if re.match(r"\s*\[",lines[i])),len(lines))
    key=f'{name} = "{constraint}"'
    for i in range(start+1,end):
        if re.match(rf"\s*{re.escape(name)}\s*=",lines[i]): lines[i]=key; break
    else: lines.insert(end,key)
    path.write_text("\n".join(lines)+"\n")

def project_root():
    return Path.cwd()

def install_package(meta, expected_sha256=None):
    name = safe_name(meta["name"])
    version = safe_version(meta["version"])
    path = safe_registry_path(meta.get("path", name))
    commit = meta.get("commit")
    if commit is not None and not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise PjpmError(f"{name}: expected an immutable commit SHA")
    repo = registry_repo()
    ref = commit if commit else "refs/heads/main"
    archive = http(f"https://codeload.github.com/{repo}/zip/{ref}", binary=True)
    digest = hashlib.sha256(archive).hexdigest()
    if expected_sha256 and digest != expected_sha256:
        raise PjpmError(f"integrity mismatch for {name}@{version}: source archive digest differs from lockfile")
    record = {"name": name, "version": version, "registry": REGISTRY,
              "commit": commit, "path": path, "sha256": digest}
    dest = Path(".pjpm/packages") / name / version
    marker = dest / ".pjpm-source.json"
    if dest.is_symlink():
        raise PjpmError(f"unsafe installation directory: {dest}")
    if dest.exists() and marker.is_file():
        try:
            current = json.loads(marker.read_text(encoding="utf-8"))
            if current == record:
                print(f"{name} {version} already installed")
                return record
        except (ValueError, OSError):
            pass

    # Extract into a temporary sibling. An incomplete extraction must not
    # remove a previously installed version.
    dest.parent.mkdir(parents=True, exist_ok=True)
    staging = Path(tempfile.mkdtemp(prefix=".pjpm-stage-", dir=dest.parent))
    try:
        with zipfile.ZipFile(io.BytesIO(archive)) as z:
            prefix = None
            source_parts = PurePosixPath(path).parts
            entries = z.infolist()
            if len(entries) > 20000 or sum(item.file_size for item in entries) > 512 * 1024 * 1024:
                raise PjpmError("registry archive exceeds safety limits")
            for item in entries:
                parts = PurePosixPath(item.filename).parts
                if item.filename.startswith("/") or ".." in parts:
                    raise PjpmError("registry archive contains an unsafe path")
                if len(parts) > len(source_parts) and parts[1:1+len(source_parts)] == source_parts:
                    prefix = "/".join(parts[:1+len(source_parts)]) + "/"
                    break
            if prefix is None:
                raise PjpmError(f"package directory {path!r} not found at source {ref}")
            for item in entries:
                if not item.filename.startswith(prefix):
                    continue
                rel = PurePosixPath(item.filename[len(prefix):])
                if not rel.parts:
                    continue
                target = (staging / Path(*rel.parts)).resolve()
                if staging.resolve() not in target.parents and target != staging.resolve():
                    raise PjpmError("archive path escapes install directory")
                if item.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with z.open(item) as source, target.open("wb") as output:
                        shutil.copyfileobj(source, output)
        (staging / ".pjpm-source.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        if dest.exists():
            shutil.rmtree(dest)
        staging.rename(dest)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    print(f"installed {name} {version} -> {dest}")
    return record

def command_for(adapter, entry=None, args=None):
    args=list(args or [])
    if adapter in ("python","python3"): return [shutil.which("python3") or sys.executable, *([entry] if entry else []), *args]
    if adapter in ("node","javascript","js"): return ["node", *([entry] if entry else []), *args]
    if adapter in ("payjar","pj"):
        executable = "./PJC" if Path("PJC").is_file() else "PJC"
        return [executable, *([entry] if entry else []), *args]
    if adapter in ("rust","cargo"): return ["cargo","run","--",*args]
    if adapter=="go": return ["go","run",*([entry] if entry else []),*args]
    if adapter in ("c","cpp","c++","cmake"): return ["cmake","--build","build","--",*args]
    if adapter in ("java","gradle"): return ["./gradlew","run",*args] if Path("gradlew").exists() else ["gradle","run",*args]
    raise PjpmError(f"no built-in adapter for {adapter!r}; use [build].adapter='custom' and an explicit [build].command array")

def run_vector(vector, cwd=Path("."), approved=False):
    if not isinstance(vector,list) or not vector or not all(isinstance(x,str) for x in vector):
        raise PjpmError("command must be a non-empty array of strings")
    exe=shutil.which(vector[0]) if not Path(vector[0]).is_absolute() else vector[0]
    if not exe: raise PjpmError(f"required tool not found: {vector[0]} (run pjpm doctor)")
    print("$ "+" ".join(vector))
    return subprocess.run([exe,*vector[1:]],cwd=cwd,check=False).returncode

def cmd_init(args):
    name=safe_name(args.name or Path.cwd().name)
    if Path(MANIFEST).exists(): raise PjpmError(f"{MANIFEST} already exists")
    Path(MANIFEST).write_text(f'''[package]
schema = 2
name = "{name}"
version = "0.1.0"
description = ""
kind = "library"
language = "python"

[runtime]
adapter = "python"
entry = "src/main.py"

[dependencies]
''')
    print(f"created {MANIFEST}")

def cmd_create(args):
    name=safe_name(args.name); Path(name).mkdir(parents=True,exist_ok=True)
    old=Path.cwd()
    os.chdir(name)
    try: cmd_init(argparse.Namespace(name=name))
    finally: os.chdir(old)
    print(f"created project ./{name}")

def resolve_graph(root_dependencies, locked=None):
    """Choose compatible releases with backtracking, then return dependencies first."""
    if not isinstance(root_dependencies, dict):
        raise PjpmError("dependencies must be a table")
    locked = locked or {}
    requirements = {safe_name(name): [str(constraint)] for name, constraint in root_dependencies.items()}
    cached = {}
    errors = []

    def candidates(name):
        if name not in cached:
            choices = registry_candidates(name)
            old = locked.get(name)
            if old:
                if old.get("registry") not in (None, REGISTRY):
                    raise PjpmError(f"{name}: lockfile registry differs from PJPM_REGISTRY")
                if old.get("commit"):
                    pinned = read_registry_manifest(name, old["commit"], old.get("path", name))
                    if pinned["version"] != old.get("version"):
                        raise PjpmError(f"{name}: locked version does not match pinned manifest")
                    choices = [pinned] + [x for x in choices if (x["version"], x.get("commit")) != (pinned["version"], pinned.get("commit"))]
                else:
                    choices.sort(key=lambda item: item["version"] != old.get("version"))
            cached[name] = choices
        return cached[name]

    def topological(selected):
        visiting, visited, ordered = [], set(), []

        def visit(name):
            if name in visiting:
                raise PjpmError("dependency cycle detected: " + " -> ".join(visiting + [name]))
            if name in visited:
                return
            visiting.append(name)
            for child in sorted(selected[name].get("dependencies", {})):
                visit(child)
            visiting.pop()
            visited.add(name)
            ordered.append(selected[name])

        for name in sorted(selected):
            visit(name)
        return ordered

    def solve(selected, needed):
        for name, meta in selected.items():
            if not all(version_ok(meta["version"], constraint) for constraint in needed.get(name, [])):
                errors.append(f"dependency conflict for {name}: selected {meta['version']} violates a constraint")
                return None
        remaining = sorted(name for name in needed if name not in selected)
        if not remaining:
            try:
                return topological(selected)
            except PjpmError as e:
                errors.append(str(e))
                return None
        name = remaining[0]
        for meta in candidates(name):
            if not all(version_ok(meta["version"], req) for req in needed[name]):
                continue
            child_needs = {n: list(reqs) for n, reqs in needed.items()}
            for child, requirement in sorted(meta.get("dependencies", {}).items()):
                child_needs.setdefault(safe_name(child), []).append(str(requirement))
            result = solve({**selected, name: meta}, child_needs)
            if result is not None:
                return result
        errors.append(f"{name}: no compatible release satisfies {', '.join(needed[name])}")
        return None

    solution = solve({}, requirements)
    if solution is None:
        raise PjpmError("; ".join(dict.fromkeys(errors[-6:])) if errors else "unable to resolve dependencies")
    return solution

def cmd_add(args):
    name, constraint = parse_spec(args.package)
    if not any(Path(filename).exists() for filename in (MANIFEST, LEGACY, "init.pj")):
        raise PjpmError(
            "pjpm add needs a project manifest in the current directory. "
            "Create a separate project directory, run 'pjpm init' there, "
            f"then run 'pjpm add {args.package}'."
        )
    manifest = load_manifest()
    if manifest["format"] != "v2":
        raise PjpmError("pjpm add requires a v2 pjpackage.toml; run pjpm init first")
    roots = dict(manifest["dependencies"])
    roots[name] = constraint
    records = [install_package(meta) for meta in resolve_graph(roots)]
    add_dependency(name, constraint)
    lock_write(records, replace_all=True, requirements=remote_dependencies(project_dependencies()))
    build_dependencies(records, getattr(args, "approve_build", False))

def lock_write(new_records, replace_all=False, requirements=None):
    p = Path(LOCK)
    current = {}
    if not replace_all and p.exists():
        current = {item["name"]: item for item in read_lock().get("packages", [])}
    for record in new_records:
        current[record["name"]] = record
    result = {"lock_version": 1, "packages": [current[key] for key in sorted(current)]}
    if requirements is not None:
        result["project_dependencies"] = requirements
    p.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


def cmd_install(args):
    roots = remote_dependencies(project_dependencies())
    existing = read_lock()
    locked = {item["name"]: item for item in existing.get("packages", [])}
    graph = resolve_graph(roots, locked=locked)
    records = []
    for meta in graph:
        name = meta["name"]
        old = locked.get(name, {})
        expected = None
        if old.get("version") == meta["version"]:
            if old.get("commit") != meta.get("commit") or old.get("path", name) != meta.get("path", name):
                raise PjpmError(f"{name}: release source changed since locking; run pjpm update to choose a new release")
            expected = old.get("sha256")
        records.append(install_package(meta, expected_sha256=expected))
    lock_write(records, replace_all=True, requirements=roots)
    build_dependencies(records, getattr(args, "approve_build", False))
    print(f"installed {len(records)} package(s), including transitive dependencies")


def build_dependencies(records, approved=False):
    """Build Cargo dependencies in lock order; Cargo handles incremental caching."""
    if not approved:
        return
    for record in records:
        root = Path(".pjpm/packages") / safe_name(record["name"]) / safe_version(record["version"])
        manifest = load_manifest(root)
        if manifest["language"] != "rust":
            continue
        build = manifest["build"]
        if build.get("adapter", "cargo") not in ("cargo", "rust"):
            raise PjpmError(f"{record['name']}: unsupported Rust build adapter")
        args = build.get("args", ["build", "--release"])
        if not isinstance(args, list) or not args or args[0] != "build" or not all(isinstance(x, str) for x in args):
            raise PjpmError(f"{record['name']}: expected Cargo build arguments")
        runtime = manifest["runtime"]
        if runtime.get("adapter") == "native":
            marker = root / ".pjpm-native.json"
            marker.unlink(missing_ok=True)
            library = runtime.get("library")
            if not isinstance(library, str) or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", library):
                raise PjpmError("invalid native library name")
            # Fixed target and profile make the installed artifact predictable.
            target = (root / "target").resolve()
            code = run_vector(["cargo", "build", "--release", "--lib", "--target-dir", str(target)], cwd=root)
            if code:
                raise PjpmError(f"{record['name']}: native build failed with exit code {code}")
            filename = library + ".dll" if sys.platform == "win32" else "lib" + library + (".dylib" if sys.platform == "darwin" else ".so")
            artifact = target / "release" / filename
            if not artifact.is_file():
                raise PjpmError(f"{record['name']}: native build produced no {filename}")
            marker.write_text(json.dumps({"library": "target/release/" + filename, "abi": 1}) + "\n", encoding="utf-8")
            continue
        code = run_vector(["cargo", *args], cwd=root)
        if code:
            raise PjpmError(f"{record['name']}: Cargo build failed with exit code {code}")


def cmd_sync(args):
    """Fast runtime sync: install only when declared dependencies or cached sources changed."""
    roots = remote_dependencies(project_dependencies())
    if not roots:
        if Path(LOCK).is_file() and read_lock().get("packages"):
            lock_write([], replace_all=True, requirements={})
        return
    if not packages_ready(roots):
        cmd_install(args)
    else:
        build_dependencies(read_lock().get("packages", []), getattr(args, "approve_build", False))


def cmd_update(args):
    roots = remote_dependencies(project_dependencies())
    locked = {}
    if args.package:
        name = safe_name(args.package)
        if name not in roots:
            raise PjpmError(f"{name} is not a registry dependency")
        locked = {item["name"]: item for item in read_lock().get("packages", [])}
        locked.pop(name, None)
    graph = resolve_graph(roots, locked=locked)
    records = [install_package(meta) for meta in graph]
    lock_write(records, replace_all=True, requirements=roots)
    build_dependencies(records, getattr(args, "approve_build", False))
    print(f"updated {len(records)} package(s)")


def cmd_remove(args):
    name=safe_name(args.package)
    m=load_manifest()
    if name not in m["dependencies"]: raise PjpmError(f"{name} is not a dependency")
    text=Path(MANIFEST).read_text()
    text=re.sub(rf"(?m)^\s*{re.escape(name)}\s*=.*\n?","",text)
    Path(MANIFEST).write_text(text)
    shutil.rmtree(Path(".pjpm/packages")/name,ignore_errors=True)
    p=Path(LOCK)
    if p.exists():
        d=json.loads(p.read_text()); d["packages"]=[x for x in d.get("packages",[]) if x["name"]!=name]; p.write_text(json.dumps(d,indent=2)+"\n")
    print(f"removed {name}")

def cmd_list(args):
    base=Path(".pjpm/packages")
    if not base.exists(): print("No packages installed."); return
    for p in sorted(base.glob("*/*")):
        if p.is_dir(): print(f"{p.parent.name} {p.name}")

def cmd_search(args):
    """Search package names, optionally filtering by a version constraint."""
    query, constraint = args.query.rsplit("@", 1) if "@" in args.query else (args.query, "*")
    if not query.strip() or not constraint.strip():
        raise PjpmError("search expects a package keyword or name@version (for example Geometry@1.0)")
    found = False
    for name in registry_dirs():
        if query.lower() not in name.lower():
            continue
        try:
            for meta in registry_candidates(name):
                if version_ok(meta["version"], constraint):
                    print(f'{meta["name"]} {meta["version"]} [{meta["language"]}]')
                    found = True
        except PjpmError as exc:
            print(f"pjpm: warning: cannot inspect {name}: {exc}", file=sys.stderr)
    if not found:
        print("No matching package versions found.")


def cmd_info(args):
    name, constraint = parse_spec(args.package)
    choices = registry_candidates(name)
    selected = next((meta for meta in choices if version_ok(meta["version"], constraint)), None)
    if selected is None:
        raise PjpmError(f"{name}: no version satisfies {constraint!r}; available: {', '.join(x['version'] for x in choices)}")
    print(json.dumps({**selected, "available_versions": [x["version"] for x in choices]}, indent=2))


def cmd_build(args):
    m=load_manifest(); b=m["build"]; adapter=b.get("adapter",m["language"])
    if adapter=="custom":
        vector=b.get("command")
    elif adapter in ("cargo", "rust"):
        vector=["cargo", *b.get("args", ["build", "--release"])]
    else:
        vector=command_for(adapter,args=b.get("args",[]))
    if not args.approve_build:
        answer=input("This runs the package build command. Continue? [y/N] ").strip().lower()
        if answer not in ("y","yes"): print("cancelled"); return
    code=run_vector(vector)
    if code: raise PjpmError(f"build failed with exit code {code}")

def cmd_run(args):
    m=load_manifest(); r=m["runtime"]; adapter=r.get("adapter",m["language"]); entry=r.get("entry")
    if adapter=="custom": vector=r.get("command")
    else: vector=command_for(adapter,entry,args.args)
    code=run_vector(vector)
    if code: raise PjpmError(f"program exited with code {code}")

def cmd_doctor(args):
    for tool in ("python3","node","cargo","go","java","cmake","PJC"):
        print(f"{tool}: {shutil.which(tool) or 'not found'}")
    print(f"Python: {sys.version.split()[0]} (pjpm requires 3.11+)")

def cmd_pack(args):
    m=load_manifest(); out=Path(f'{m["name"]}-{m["version"]}.zip')
    with zipfile.ZipFile(out,"w",zipfile.ZIP_DEFLATED) as z:
        for p in sorted(Path(".").rglob("*")):
            if not p.is_file() or any(x in (".git",".pjpm","target","node_modules","__pycache__") for x in p.parts): continue
            z.write(p,p.as_posix())
    digest=hashlib.sha256(out.read_bytes()).hexdigest()
    print(f"created {out} sha256:{digest}")

def main():
    ap=argparse.ArgumentParser(prog="pjpm",description="Payjar universal package manager")
    ap.add_argument("--version",action="version",version=f"pjpm {VERSION}")
    sub=ap.add_subparsers(dest="cmd",required=True)
    p=sub.add_parser("init"); p.add_argument("name",nargs="?"); p.set_defaults(fn=cmd_init)
    p=sub.add_parser("create"); p.add_argument("name"); p.set_defaults(fn=cmd_create)
    p=sub.add_parser("add"); p.add_argument("package"); p.add_argument("--approve-build",action="store_true",help="Allow Cargo dependencies to execute build code"); p.set_defaults(fn=cmd_add)
    p=sub.add_parser("install"); p.add_argument("--approve-build",action="store_true",help="Allow Cargo dependencies to execute build code"); p.set_defaults(fn=cmd_install)
    p=sub.add_parser("sync"); p.add_argument("--approve-build",action="store_true",help="Allow Cargo dependencies to execute build code"); p.set_defaults(fn=cmd_sync)
    p=sub.add_parser("update"); p.add_argument("package",nargs="?"); p.add_argument("--approve-build",action="store_true"); p.set_defaults(fn=cmd_update)
    p=sub.add_parser("remove"); p.add_argument("package"); p.set_defaults(fn=cmd_remove)
    p=sub.add_parser("uninstall"); p.add_argument("package"); p.set_defaults(fn=cmd_remove)
    p=sub.add_parser("list"); p.set_defaults(fn=cmd_list)
    p=sub.add_parser("search"); p.add_argument("query"); p.set_defaults(fn=cmd_search)
    p=sub.add_parser("info"); p.add_argument("package"); p.set_defaults(fn=cmd_info)
    p=sub.add_parser("build"); p.add_argument("--approve-build",action="store_true"); p.set_defaults(fn=cmd_build)
    p=sub.add_parser("run"); p.add_argument("args",nargs=argparse.REMAINDER); p.set_defaults(fn=cmd_run)
    p=sub.add_parser("doctor"); p.set_defaults(fn=cmd_doctor)
    p=sub.add_parser("pack"); p.set_defaults(fn=cmd_pack)
    args=ap.parse_args()
    try: args.fn(args)
    except (PjpmError,OSError,ValueError) as e: print(f"pjpm: error: {e}",file=sys.stderr); return 1
    return 0

if __name__=="__main__": raise SystemExit(main())
