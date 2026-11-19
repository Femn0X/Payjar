//! Connects runtime imports to the pjpm project lockfile.
//! Packages remain source files: downloading them does not transpile other languages.
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn project_root_for_script(script_path: &Path) -> PathBuf {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if cwd.join("config.toml").is_file() {
        return cwd;
    }
    if let Some(parent) = script_path.parent() {
        let candidate = if parent.is_absolute() { parent.to_path_buf() } else { cwd.join(parent) };
        if candidate.join("config.toml").is_file() {
            return candidate;
        }
    }
    cwd
}

fn has_declared_config_dependencies(project_root: &Path) -> Result<bool, String> {
    let config = project_root.join("config.toml");
    if !config.exists() {
        return Ok(false);
    }
    let text = fs::read_to_string(&config)
        .map_err(|err| format!("Cannot read {}: {}", config.display(), err))?;
    // tomllib in pjpm is the authoritative parser. This cheap check avoids
    // requiring Python for unrelated runtime config files.
    Ok(text.lines().any(|line| line.trim() == "[dependencies]"))
}

fn find_pjpm_script(project_root: &Path) -> Option<PathBuf> {
    if let Some(override_path) = env::var_os("PAYJAR_PJPM") {
        return Some(PathBuf::from(override_path));
    }
    let candidate = project_root.join("pjpm.py");
    if candidate.is_file() {
        return Some(candidate);
    }
    if let Ok(cwd) = env::current_dir() {
        let candidate = cwd.join("pjpm.py");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    if let Ok(binary) = env::current_exe() {
        for ancestor in binary.ancestors().skip(1) {
            let candidate = ancestor.join("pjpm.py");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Called before loading imports, only for projects declaring config dependencies.
/// `pjpm sync` makes no network requests when the lock and local sources match.
pub fn sync_config_dependencies(project_root: &Path) -> Result<(), String> {
    if !has_declared_config_dependencies(project_root)? {
        return Ok(());
    }
    let script = find_pjpm_script(project_root);
    let mut cmd = if let Some(ref script) = script {
        let python = env::var_os("PAYJAR_PYTHON")
            .unwrap_or_else(|| if cfg!(windows) { "python".into() } else { "python3".into() });
        let mut command = Command::new(python);
        command.arg(script);
        command
    } else {
        Command::new("pjpm")
    };
    let result = cmd.arg("sync").current_dir(project_root).status().map_err(|err| {
        format!("Unable to run pjpm for {}: {}. Install Python 3.11+ and make pjpm available; or set PAYJAR_PJPM to pjpm.py.",
                project_root.display(), err)
    })?;
    if result.success() {
        Ok(())
    } else {
        Err(format!("pjpm sync failed in {} (exit code {:?}); cannot resolve declared dependencies",
                    project_root.display(), result.code()))
    }
}

fn safe_lock_version(version: &str) -> bool {
    !version.is_empty()
        && !matches!(version, "." | "..")
        && version.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-' | b'+'))
}

pub fn package_init_path(project_root: &Path, name: &str) -> Result<PathBuf, String> {
    // Explicit local package sources win over anything from the package manager.
    let local = project_root.join(name).join("init.pj");
    if local.is_file() {
        return Ok(local);
    }
    let lock_path = project_root.join("pjpm.lock");
    if !lock_path.is_file() {
        return Err(format!("package '{}' not found locally and {} does not exist; add it under [dependencies] in config.toml",
                           name, lock_path.display()));
    }
    let lock: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&lock_path)
            .map_err(|err| format!("Cannot read {}: {}", lock_path.display(), err))?
    ).map_err(|err| format!("Invalid {}: {}", lock_path.display(), err))?;
    let record = lock.get("packages").and_then(|packages| packages.as_array())
        .and_then(|packages| packages.iter().find(|entry| entry.get("name").and_then(|s| s.as_str()) == Some(name)))
        .ok_or_else(|| format!("package '{}' not found locally or in {}", name, lock_path.display()))?;
    let version = record.get("version").and_then(|value| value.as_str())
        .filter(|version| safe_lock_version(version))
        .ok_or_else(|| format!("Invalid locked version for package '{}'", name))?;
    let installed = project_root.join(".pjpm").join("packages").join(name).join(version).join("init.pj");
    if installed.is_file() {
        return Ok(installed);
    }
    Err(format!("package '{}' is installed as {} but has no Payjar init.pj; direct import requires Payjar source (other languages need a runtime bridge)",
                name, project_root.join(".pjpm/packages").join(name).join(version).display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn local_package_has_priority() {
        let dir = tempdir().unwrap();
        let local = dir.path().join("Demo/init.pj");
        fs::create_dir_all(local.parent().unwrap()).unwrap();
        fs::write(&local, "package Demo;").unwrap();
        assert_eq!(package_init_path(dir.path(), "Demo").unwrap(), local);
    }

    #[test]
    fn finds_installed_package_by_locked_version() {
        let dir = tempdir().unwrap();
        let installed = dir.path().join(".pjpm/packages/Demo/1.2.3/init.pj");
        fs::create_dir_all(installed.parent().unwrap()).unwrap();
        fs::write(&installed, "package Demo;").unwrap();
        fs::write(dir.path().join("pjpm.lock"),
                  r#"{"packages":[{"name":"Demo","version":"1.2.3"}]}"#).unwrap();
        assert_eq!(package_init_path(dir.path(), "Demo").unwrap(), installed);
    }

    #[test]
    fn interpreter_loads_a_pjpm_installed_payjar_module() {
        let dir = tempdir().unwrap();
        let installed = dir.path().join(".pjpm/packages/Geometry/1.0.1/init.pj");
        fs::create_dir_all(installed.parent().unwrap()).unwrap();
        fs::write(&installed, "package Geometry;\nfunc hypot(a:Float,b:Float)->Float { return math.sqrt(a*a+b*b); }\n").unwrap();
        fs::write(dir.path().join("pjpm.lock"),
                  r#"{"packages":[{"name":"Geometry","version":"1.0.1"}]}"#).unwrap();
        let mut env = super::super::Env::new();
        env.package_root = dir.path().to_path_buf();
        super::super::load_folder_package(&mut env, "Geometry", None, false);
        assert!(env.funcs.contains_key("hypot"));
    }

    #[test]
    fn prevents_unsafe_version_paths() {
        assert!(!safe_lock_version("../outside"));
        assert!(!safe_lock_version("1/../../bad"));
        assert!(!safe_lock_version(".."));
        assert!(safe_lock_version("1.2.3-rc.1+build"));
    }

    #[test]
    fn ignores_runtime_config_without_dependencies() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "backend = \"interpreter\"\n").unwrap();
        assert!(!has_declared_config_dependencies(dir.path()).unwrap());
        sync_config_dependencies(dir.path()).unwrap();
    }
}
