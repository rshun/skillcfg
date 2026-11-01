use skill_core::PackageTree;
use skill_core::{EntryKind, TreeEntry};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}")]
pub struct IoError {
    pub code: &'static str,
}

#[derive(Clone, Copy)]
pub struct ScanLimits {
    pub file_bytes: usize,
    pub package_bytes: usize,
    pub entries: usize,
    pub depth: usize,
}
impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            file_bytes: 4 * 1024 * 1024,
            package_bytes: 16 * 1024 * 1024,
            entries: 1024,
            depth: 16,
        }
    }
}

#[derive(Clone)]
pub struct PackageSnapshot {
    pub source: PathBuf,
    pub actual_path: PathBuf,
    pub physical_key: String,
    pub root_was_link: bool,
    pub tree: PackageTree,
}

pub fn read_bounded_file(path: &Path, limit: usize) -> Result<Vec<u8>, IoError> {
    if !fs::symlink_metadata(path)
        .map_err(crate::paths::error)?
        .is_file()
    {
        return Err(IoError {
            code: "not_regular_file",
        });
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Nonblocking prevents a replacement FIFO from hanging open; no-follow
        // rejects a final-component symlink swapped in after the metadata check.
        #[cfg(target_os = "linux")]
        options.custom_flags(0x800 | 0x20000); // O_NONBLOCK | O_NOFOLLOW
        #[cfg(target_os = "macos")]
        options.custom_flags(0x4 | 0x100); // O_NONBLOCK | O_NOFOLLOW
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000); // FILE_FLAG_OPEN_REPARSE_POINT
    }
    let file = options.open(path).map_err(crate::paths::error)?;
    let metadata = file.metadata().map_err(crate::paths::error)?;
    if !metadata.is_file() {
        return Err(IoError {
            code: "not_regular_file",
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(IoError {
                code: "not_regular_file",
            });
        }
    }
    if metadata.len() > limit as u64 {
        return Err(IoError {
            code: "file_too_large",
        });
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(crate::paths::error)?;
    if bytes.len() > limit {
        return Err(IoError {
            code: "file_too_large",
        });
    }
    Ok(bytes)
}

fn capture(root: &Path, limits: &ScanLimits) -> Result<PackageSnapshot, IoError> {
    use crate::paths::{error, is_link, physical_key};
    let root_was_link = is_link(root)?;
    let actual = fs::canonicalize(root).map_err(error)?;
    if !actual.is_dir() {
        return Err(IoError {
            code: "not_a_directory",
        });
    }
    let key = physical_key(root)?;
    let mut entries = Vec::new();
    let mut total = 0usize;
    let mut pending = vec![(actual.clone(), 0usize)];
    while let Some((dir, depth)) = pending.pop() {
        if depth > limits.depth {
            return Err(IoError { code: "too_deep" });
        }
        for child in fs::read_dir(&dir).map_err(error)? {
            let child = child.map_err(error)?;
            let path = child.path();
            if is_link(&path)? {
                return Err(IoError {
                    code: "internal_link",
                });
            }
            let actual_child = fs::canonicalize(&path).map_err(error)?;
            if !actual_child.starts_with(&actual) {
                return Err(IoError {
                    code: "path_escape",
                });
            }
            let relative = path
                .strip_prefix(&actual)
                .map_err(|_| IoError {
                    code: "path_escape",
                })?
                .to_str()
                .ok_or(IoError {
                    code: "non_utf8_path",
                })?
                .to_owned();
            #[cfg(windows)]
            let relative = relative.replace('\\', "/");
            #[cfg(unix)]
            if relative.contains('\\') {
                return Err(IoError {
                    code: "invalid_path",
                });
            }
            if relative.split('/').count() > limits.depth {
                return Err(IoError { code: "too_deep" });
            }
            if entries.len() >= limits.entries {
                return Err(IoError {
                    code: "too_many_entries",
                });
            }
            let meta = fs::symlink_metadata(&path).map_err(error)?;
            let kind = if meta.is_dir() {
                pending.push((path.clone(), depth + 1));
                EntryKind::Directory
            } else if meta.is_file() {
                let bytes = read_bounded_file(&path, limits.file_bytes)?;
                total = total.saturating_add(bytes.len());
                if total > limits.package_bytes {
                    return Err(IoError {
                        code: "package_too_large",
                    });
                }
                #[cfg(unix)]
                let executable = {
                    use std::os::unix::fs::PermissionsExt;
                    meta.permissions().mode() & 0o111 != 0
                };
                #[cfg(windows)]
                let executable = false;
                EntryKind::File {
                    bytes,
                    unix_executable: executable,
                }
            } else {
                return Err(IoError {
                    code: "special_file",
                });
            };
            if is_link(&path)? || fs::canonicalize(&path).map_err(error)? != actual_child {
                return Err(IoError {
                    code: "snapshot_changed",
                });
            }
            entries.push(TreeEntry {
                relative_path: relative,
                kind,
            });
        }
    }
    entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    if physical_key(root)? != key {
        return Err(IoError {
            code: "snapshot_changed",
        });
    }
    Ok(PackageSnapshot {
        source: root.to_owned(),
        actual_path: actual,
        physical_key: key,
        root_was_link,
        tree: PackageTree { entries },
    })
}

pub fn read_package_snapshot(root: &Path, limits: &ScanLimits) -> Result<PackageSnapshot, IoError> {
    let max = ScanLimits::default();
    if !root.is_absolute()
        || limits.file_bytes > max.file_bytes
        || limits.package_bytes > max.package_bytes
        || limits.entries > max.entries
        || limits.depth > max.depth
    {
        return Err(IoError {
            code: "invalid_limits",
        });
    }
    let first = capture(root, limits)?;
    let second = capture(root, limits)?;
    if first.physical_key != second.physical_key
        || first.actual_path != second.actual_path
        || first.tree != second.tree
        || first.root_was_link != second.root_was_link
    {
        return Err(IoError {
            code: "snapshot_changed",
        });
    }
    Ok(first)
}
