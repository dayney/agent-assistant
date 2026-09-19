use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn home_dir_from(override_home: Option<OsString>) -> Result<PathBuf, String> {
    let home = match override_home {
        Some(value) if !value.is_empty() => PathBuf::from(value),
        Some(_) => return Err("home directory override is empty".to_string()),
        None => dirs::home_dir().ok_or_else(|| "home directory is unavailable".to_string())?,
    };
    Ok(home)
}

pub(crate) fn agentsync_home(home: &Path) -> PathBuf {
    home.join(".agentsync")
}

pub(crate) fn project_home(project: &Path) -> PathBuf {
    project.join(".agentsync")
}

pub(crate) fn reject_symlinks_below(root: &Path, path: &Path) -> Result<(), String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return Err(format!("{} escapes {}", path.display(), root.display()));
        };
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!("refusing symlink path {}", current.display()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(format!("inspect {}: {error}", current.display())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{agentsync_home, home_dir_from, project_home};
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn paths_are_built_with_pathbuf_components() {
        let home = home_dir_from(Some(OsString::from("/tmp/example-home"))).unwrap();
        assert_eq!(home, PathBuf::from("/tmp/example-home"));
        assert_eq!(agentsync_home(&home), home.join(".agentsync"));
        assert_eq!(
            project_home(&home.join("repo")),
            home.join("repo/.agentsync")
        );
    }

    #[test]
    fn empty_override_is_rejected() {
        let error = home_dir_from(Some(OsString::new())).unwrap_err();
        assert!(error.contains("home directory"));
    }
}
