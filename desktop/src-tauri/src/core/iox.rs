use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);
const PUBLIC_FILE_MODE: u32 = 0o644;
const PRIVATE_FILE_MODE: u32 = 0o600;
const LOCK_TIMEOUT: Duration = Duration::from_secs(30);
const LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);

pub(crate) fn atomic_write(path: &Path, data: &[u8]) -> io::Result<()> {
    atomic_write_with_mode(path, data, PUBLIC_FILE_MODE, || Ok(()))
}

pub(crate) fn atomic_write_private(path: &Path, data: &[u8]) -> io::Result<()> {
    atomic_write_with_mode(path, data, PRIVATE_FILE_MODE, || Ok(()))
}

pub(crate) fn with_exclusive_lock<T, F>(path: &Path, operation: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    with_exclusive_lock_timeout(path, LOCK_TIMEOUT, operation)
}

fn with_exclusive_lock_timeout<T, F>(
    path: &Path,
    timeout: Duration,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let parent = path
        .parent()
        .ok_or_else(|| format!("lock path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("create lock directory {}: {error}", parent.display()))?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("open lock {}: {error}", path.display()))?;
    let mut lock = fd_lock::RwLock::new(file);
    let started = Instant::now();
    let _guard = loop {
        match lock.try_write() {
            Ok(guard) => break guard,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                let elapsed = started.elapsed();
                if elapsed >= timeout {
                    return Err(format!(
                        "lock {} busy after {} ms",
                        path.display(),
                        timeout.as_millis()
                    ));
                }
                std::thread::sleep(LOCK_RETRY_INTERVAL.min(timeout - elapsed));
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => {
                return Err(format!("acquire lock {}: {error}", path.display()));
            }
        }
    };
    operation()
}

#[cfg(test)]
fn atomic_write_with<F>(path: &Path, data: &[u8], before_rename: F) -> io::Result<()>
where
    F: FnOnce() -> io::Result<()>,
{
    atomic_write_with_mode(path, data, PUBLIC_FILE_MODE, before_rename)
}

fn atomic_write_with_mode<F>(
    path: &Path,
    data: &[u8],
    mode: u32,
    before_rename: F,
) -> io::Result<()>
where
    F: FnOnce() -> io::Result<()>,
{
    reject_symlink_destination(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp_path = next_temp_path(path)?;
    let result = (|| {
        let mut temp = create_temp_file(&temp_path)?;
        temp.write_all(data)?;
        temp.sync_all()?;
        before_rename()?;
        drop(temp);
        replace_file(&temp_path, path)?;
        set_file_mode(path, mode)?;
        sync_parent(parent)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result
}

fn reject_symlink_destination(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing to replace symlink destination {}", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn create_temp_file(path: &Path) -> io::Result<std::fs::File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)?;
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))?;
    Ok(file)
}

#[cfg(not(unix))]
fn create_temp_file(path: &Path) -> io::Result<std::fs::File> {
    OpenOptions::new().create_new(true).write(true).open(path)
}

#[cfg(unix)]
fn set_file_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

#[cfg(not(unix))]
fn set_file_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}

fn next_temp_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no file name"))?
        .to_string_lossy();
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    Ok(parent.join(format!(".{file_name}.tmp-{}-{id}", std::process::id())))
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    if !destination.exists() {
        return fs::rename(source, destination);
    }

    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }

    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let source: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replaced = unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            source.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> io::Result<()> {
    std::fs::File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::atomic_write_private;
    use super::{atomic_write, atomic_write_with, with_exclusive_lock_timeout};
    use crate::core::test_support::TestDir;
    use std::io;
    use std::time::Duration;

    #[test]
    fn atomic_write_replaces_existing_content() {
        let dir = TestDir::new("atomic-replace");
        let path = dir.path().join("nested/config.json");
        atomic_write(&path, b"old").unwrap();
        atomic_write(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o644);
        }
    }

    #[test]
    fn failure_before_rename_preserves_target() {
        let dir = TestDir::new("atomic-crash");
        let path = dir.path().join("config.json");
        std::fs::write(&path, b"stable").unwrap();

        let error = atomic_write_with(&path, b"partial", || {
            Err(io::Error::other("simulated crash"))
        })
        .unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(std::fs::read(path).unwrap(), b"stable");
    }

    #[cfg(unix)]
    #[test]
    fn atomic_write_rejects_symlink_destinations() {
        use std::os::unix::fs::symlink;

        let dir = TestDir::new("atomic-symlink");
        let target = dir.path().join("target.json");
        let link = dir.path().join("config.json");
        std::fs::write(&target, b"stable").unwrap();
        symlink(&target, &link).unwrap();

        let error = atomic_write(&link, b"replacement").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(std::fs::read(target).unwrap(), b"stable");
    }

    #[cfg(unix)]
    #[test]
    fn private_atomic_write_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TestDir::new("atomic-private");
        let path = dir.path().join("backup.md");

        atomic_write_private(&path, b"sensitive backup").unwrap();

        let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn exclusive_lock_times_out_when_busy() {
        let dir = TestDir::new("lock-timeout");
        let path = dir.path().join("agentsync.lock");

        with_exclusive_lock_timeout(&path, Duration::from_secs(1), || {
            let error = with_exclusive_lock_timeout(&path, Duration::from_millis(40), || Ok(()))
                .unwrap_err();
            assert!(error.contains("busy after"), "unexpected error: {error}");
            Ok(())
        })
        .unwrap();
    }
}
