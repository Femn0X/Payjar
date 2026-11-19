#!/usr/bin/env python3
"""Payjar package manager (pjpm). Requires Python 3.11+; standard library only."""
from __future__ import annotations
import argparse, hashlib, json, os, re, shutil, subprocess, sys, tempfile, tomllib, urllib.error, urllib.parse, urllib.request, zipfile
from pathlib import Path, PurePosixPath

VERSION = "0.2.0"
REGISTRY = os.environ.get("PJPM_REGISTRY", "https://github.com/Femn0X/Payjar-Packages")
API = "https://api.github.com"
LOCK = "pjpm.lock"
MANIFEST = "pjpackage.toml"
LEGACY = "package.toml"

class PjpmError(Exception): pass

def http(url, binary=False):
    req = urllib.request.Request(url, headers={"User-Agent":"pjpm/0.1", "Accept":"application/vnd.github+json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as res:
            return res.read() if binary else res.read().decode("utf-8")
    except Exception as e:
        raise PjpmError(f"request failed: {url}: {e}") from e

def safe_name(name):
    if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9._-]*", name) or name in (".", ".."):
        raise PjpmError(f"invalid package name: {name!r}")
    return name

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
        if not re.fullmatch(r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", p["version"]):
            raise PjpmError("package.version must be SemVer (for example 1.2.3)")
        p["kind"] = p.get("kind", "library")
        return {"format":"v2", "name":p["name"], "version":p["version"], "language":p["language"].lower(),
                "description":p.get("description",""), "kind":p["kind"], "build":data.get("build",{}),
                "runtime":data.get("runtime",{}), "dependencies":data.get("dependencies",{}), "raw":data}
    if old.exists():
        try: d = tomllib.loads(old.read_text(encoding="utf-8"))
        except Exception as e: raise PjpmError(f"invalid legacy package.toml: {e}")
        name = root.resolve().name
        return {"format":"legacy", "name":name, "version":str(d.get("version","0.0.0")),
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
    dirs=set()
    for item in data.get("tree",[]):
        parts=item.get("path","").split("/")
        if len(parts)>1 and item.get("type")=="blob": dirs.add(parts[0])
    return sorted(d for d in dirs if d not in (".github",))

def registry_manifest(name):
    repo=registry_repo()
    for filename in (MANIFEST, LEGACY):
        url=f"https://raw.githubusercontent.com/{repo}/main/{urllib.parse.quote(name)}/{filename}"
        try: d=tomllib.loads(http(url))
        except PjpmError: continue
        if filename==MANIFEST:
            p=d.get("package",{})
            return {"name":p.get("name",name),"version":p.get("version","0.0.0"),"language":p.get("language","unknown"),"manifest":filename,"dependencies":d.get("dependencies",{})}
        return {"name":name,"version":str(d.get("version","0.0.0")),"language":d.get("language","payjar"),"manifest":filename}
    raise PjpmError(f"package {name!r} has no readable manifest in registry")

def parse_spec(spec):
    if "@" in spec:
        name, constraint=spec.rsplit("@",1)
        return safe_name(name), constraint or "*"
    return safe_name(spec), "*"

def version_ok(version, constraint):
    if constraint in ("","*","latest"): return True
    def nums(v):
        m=re.match(r"^(\d+)\.(\d+)\.(\d+)",v)
        return tuple(map(int,m.groups())) if m else None
    v=nums(version)
    if not v: return version==constraint
    for part in (x.strip() for x in constraint.split(",")):
        if not part or part=="*": continue
        op=next((x for x in (">=","<=","!=",">","<","^","~","=") if part.startswith(x)), "=")
        raw=part[len(op):].strip()
        b=nums(raw)
        if not b: return version==constraint
        if op=="^": ok=v>=b and (v[0]==b[0] if b[0]>0 else (v[1]==b[1] if b[1]>0 else v[2]==b[2]))
        elif op=="~": ok=v>=b and v[:2]==b[:2]
        elif op==">=": ok=v>=b
        elif op=="<=": ok=v<=b
        elif op==">": ok=v>b
        elif op=="<": ok=v<b
        elif op=="!=": ok=v!=b
        else: ok=v==b
        if not ok: return False
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

def install_package(name, constraint="*"):
    meta=registry_manifest(name)
    if not version_ok(meta["version"],constraint):
        raise PjpmError(f"{name}: registry has {meta['version']}, which does not satisfy {constraint}; versioned registry releases are not available yet")
    repo=registry_repo()
    archive=http(f"https://codeload.github.com/{repo}/zip/refs/heads/main",binary=True)
    digest=hashlib.sha256(archive).hexdigest()
    dest=Path(".pjpm/packages")/name/meta["version"]
    if dest.exists(): print(f"{name} {meta['version']} already installed"); return {"name":name,"version":meta["version"],"sha256":digest}
    dest.parent.mkdir(parents=True,exist_ok=True)
    try:
        with zipfile.ZipFile(__import__("io").BytesIO(archive)) as z:
            prefix=None
            for item in z.infolist():
                p=PurePosixPath(item.filename)
                if p.is_absolute() or ".." in p.parts: raise PjpmError("registry archive contains an unsafe path")
                if len(p.parts)>=2 and p.parts[1]==name:
                    prefix=p.parts[0]+"/"+name+"/"; break
            if prefix is None: raise PjpmError(f"package folder {name!r} not found in registry archive")
            for item in z.infolist():
                if not item.filename.startswith(prefix): continue
                rel=PurePosixPath(item.filename[len(prefix):])
                if not rel.parts: continue
                target=(dest/Path(*rel.parts)).resolve()
                if dest.resolve() not in target.parents and target!=dest.resolve(): raise PjpmError("archive path escapes install directory")
                if item.is_dir(): target.mkdir(parents=True,exist_ok=True)
                else:
                    target.parent.mkdir(parents=True,exist_ok=True)
                    with z.open(item) as src, target.open("wb") as out: shutil.copyfileobj(src,out)
    except Exception:
        shutil.rmtree(dest,ignore_errors=True); raise
    print(f"installed {name} {meta['version']} -> {dest}")
    return {"name":name,"version":meta["version"],"registry":REGISTRY,"sha256":digest}

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

def resolve_graph(root_dependencies):
    """Resolve registry dependencies and reject cycles or incompatible constraints."""
    selected = {}
    visiting = []
    ordered = []
    def visit(name, constraint):
        name = safe_name(name)
        if name in visiting:
            raise PjpmError("dependency cycle detected: " + " -> ".join(visiting + [name]))
        if name in selected:
            if not version_ok(selected[name]["version"], constraint):
                raise PjpmError(f"dependency conflict for {name}: selected {selected[name]['version']} does not satisfy {constraint}")
            return
        visiting.append(name)
        meta = registry_manifest(name)
        if not version_ok(meta["version"], constraint):
            raise PjpmError(f"{name}: registry has {meta['version']}, which does not satisfy {constraint}; only the current registry version is available")
        selected[name] = meta
        for child, child_constraint in sorted(meta.get("dependencies", {}).items()):
            visit(child, str(child_constraint))
        visiting.pop()
        ordered.append((name, constraint))
    for name, constraint in sorted(root_dependencies.items()):
        visit(name, str(constraint))
    return ordered

def cmd_add(args):
    name,constraint=parse_spec(args.package)
    records=[install_package(dep, req) for dep,req in resolve_graph({name:constraint})]
    add_dependency(name,constraint)
    lock_write(records)

def lock_write(new_records, replace_all=False):
    p=Path(LOCK)
    current={}
    if not replace_all and p.exists():
        try: current={x["name"]:x for x in json.loads(p.read_text()).get("packages",[])}
        except Exception: pass
    for r in new_records: current[r["name"]]=r
    p.write_text(json.dumps({"lock_version":1,"packages":[current[k] for k in sorted(current)]},indent=2)+"\n")

def cmd_install(args):
    m=load_manifest()
    lock=Path(LOCK)
    locked={}
    if lock.exists():
        try: locked={x["name"]:x for x in json.loads(lock.read_text()).get("packages",[])}
        except Exception as e: raise PjpmError(f"invalid {LOCK}: {e}")
    graph=resolve_graph(m["dependencies"])
    records=[]
    for name,constraint in graph:
        rec=install_package(name,constraint)
        if name in locked and locked[name].get("version")==rec["version"] and locked[name].get("sha256")!=rec["sha256"]:
            raise PjpmError(f"integrity mismatch for {name}: registry archive changed since lockfile creation")
        records.append(rec)
    lock_write(records, replace_all=True)
    print(f"installed {len(records)} package(s), including transitive dependencies")

def cmd_update(args):
    m=load_manifest()
    roots=m["dependencies"]
    if args.package:
        name=safe_name(args.package)
        if name not in roots: raise PjpmError(f"{name} is not a direct dependency")
        roots={name:roots[name]}
    graph=resolve_graph(roots)
    records=[install_package(name,constraint) for name,constraint in graph]
    lock_write(records)
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
    q=args.query.lower()
    matches=[n for n in registry_dirs() if q in n.lower()]
    for n in matches:
        try:
            m=registry_manifest(n); print(f'{m["name"]} {m["version"]} [{m["language"]}]')
        except PjpmError: print(n)
    if not matches: print("No packages found.")

def cmd_info(args):
    n=safe_name(args.package); print(json.dumps(registry_manifest(n),indent=2))

def cmd_build(args):
    m=load_manifest(); b=m["build"]; adapter=b.get("adapter",m["language"])
    if adapter=="custom":
        vector=b.get("command")
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
    p=sub.add_parser("add"); p.add_argument("package"); p.set_defaults(fn=cmd_add)
    p=sub.add_parser("install"); p.set_defaults(fn=cmd_install)
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
