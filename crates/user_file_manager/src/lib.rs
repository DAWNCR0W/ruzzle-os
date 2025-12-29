#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use user_fs_service::{FileSystem, FsError};

/// Filesystem abstraction used by the file manager.
pub trait Fs {
    fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError>;
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError>;
    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FsError>;
    fn mkdir(&mut self, path: &str) -> Result<(), FsError>;
    fn remove(&mut self, path: &str) -> Result<(), FsError>;
}

impl Fs for FileSystem {
    fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        FileSystem::list_dir(self, path)
    }

    fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        FileSystem::read_file(self, path)
    }

    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FsError> {
        FileSystem::write_file(self, path, data)
    }

    fn mkdir(&mut self, path: &str) -> Result<(), FsError> {
        FileSystem::mkdir(self, path)
    }

    fn remove(&mut self, path: &str) -> Result<(), FsError> {
        FileSystem::remove(self, path)
    }
}

/// Minimal file manager state (current working directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManager {
    cwd: String,
}

impl FileManager {
    /// Creates a file manager rooted at `/`.
    pub fn new() -> Self {
        Self {
            cwd: "/".to_string(),
        }
    }

    /// Returns the current working directory.
    pub fn pwd(&self) -> &str {
        &self.cwd
    }

    /// Resolves a path against the current working directory.
    pub fn resolve(&self, path: &str) -> Result<String, FsError> {
        resolve_path(&self.cwd, path)
    }

    /// Changes the current working directory.
    pub fn cd(&mut self, fs: &impl Fs, path: &str) -> Result<(), FsError> {
        let resolved = resolve_path(&self.cwd, path)?;
        fs.list_dir(&resolved)?;
        self.cwd = resolved;
        Ok(())
    }

    /// Lists directory entries.
    pub fn ls(&self, fs: &impl Fs) -> Result<Vec<String>, FsError> {
        fs.list_dir(&self.cwd)
    }

    /// Lists directory entries for an explicit path.
    pub fn ls_path(&self, fs: &impl Fs, path: &str) -> Result<Vec<String>, FsError> {
        let resolved = resolve_path(&self.cwd, path)?;
        fs.list_dir(&resolved)
    }

    /// Reads a file as UTF-8 text.
    pub fn cat(&self, fs: &impl Fs, path: &str) -> Result<String, FsError> {
        let resolved = resolve_path(&self.cwd, path)?;
        let data = fs.read_file(&resolved)?;
        let text = core::str::from_utf8(&data).map_err(|_| FsError::InvalidUtf8)?;
        Ok(text.to_string())
    }

    /// Writes text to a file.
    pub fn write(&self, fs: &mut impl Fs, path: &str, text: &str) -> Result<(), FsError> {
        let resolved = resolve_path(&self.cwd, path)?;
        fs.write_file(&resolved, text.as_bytes())
    }

    /// Creates a directory.
    pub fn mkdir(&self, fs: &mut impl Fs, path: &str) -> Result<(), FsError> {
        let resolved = resolve_path(&self.cwd, path)?;
        fs.mkdir(&resolved)
    }

    /// Removes a file or directory.
    pub fn rm(&self, fs: &mut impl Fs, path: &str) -> Result<(), FsError> {
        let resolved = resolve_path(&self.cwd, path)?;
        fs.remove(&resolved)
    }
}

fn resolve_path(cwd: &str, path: &str) -> Result<String, FsError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(FsError::InvalidPath);
    }
    if trimmed.contains("//") {
        return Err(FsError::InvalidPath);
    }

    let mut segments: Vec<&str> = Vec::new();
    if !trimmed.starts_with('/') && cwd != "/" {
        for segment in cwd.split('/') {
            if segment.is_empty() {
                continue;
            }
            segments.push(segment);
        }
    }

    let path_body = if trimmed == "/" { "" } else { trimmed };
    for segment in path_body.split('/') {
        if segment.is_empty() {
            continue;
        }
        match segment {
            "." => {}
            ".." => {
                segments.pop();
            }
            _ => segments.push(segment),
        }
    }

    if segments.is_empty() {
        Ok("/".to_string())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_handles_absolute_and_relative() {
        assert_eq!(resolve_path("/", "etc").unwrap(), "/etc");
        assert_eq!(resolve_path("/home", "docs").unwrap(), "/home/docs");
        assert_eq!(resolve_path("/", "/etc").unwrap(), "/etc");
        assert_eq!(resolve_path("/home", "/tmp/").unwrap(), "/tmp");
        assert_eq!(resolve_path("/home", "/").unwrap(), "/");
        assert_eq!(resolve_path("/", "/").unwrap(), "/");
    }

    #[test]
    fn resolve_method_uses_cwd() {
        let manager = FileManager::new();
        assert_eq!(manager.resolve("etc").unwrap(), "/etc");
    }

    #[test]
    fn resolve_path_rejects_empty() {
        assert_eq!(resolve_path("/", ""), Err(FsError::InvalidPath));
    }

    #[test]
    fn resolve_path_normalizes_dot_and_dotdot() {
        assert_eq!(resolve_path("/home/user", "..").unwrap(), "/home");
        assert_eq!(resolve_path("/home/user", "../docs").unwrap(), "/home/docs");
        assert_eq!(resolve_path("/home", "./docs").unwrap(), "/home/docs");
        assert_eq!(resolve_path("/", "..").unwrap(), "/");
        assert_eq!(resolve_path("/home/user", "../../..").unwrap(), "/");
    }

    #[test]
    fn resolve_path_rejects_double_slash() {
        assert_eq!(
            resolve_path("/home", "docs//notes"),
            Err(FsError::InvalidPath)
        );
    }

    #[test]
    fn file_manager_basic_flow() {
        let mut fs = FileSystem::new();
        fs.mkdir("/home").unwrap();
        let mut manager = FileManager::new();
        manager.cd(&fs, "/home").unwrap();
        assert_eq!(manager.pwd(), "/home");
        manager.write(&mut fs, "notes.txt", "hello").unwrap();
        assert_eq!(manager.cat(&fs, "notes.txt").unwrap(), "hello");
        let list = manager.ls(&fs).unwrap();
        assert_eq!(list, vec!["notes.txt".to_string()]);
        manager.rm(&mut fs, "notes.txt").unwrap();
        assert!(manager.ls(&fs).unwrap().is_empty());
    }

    #[test]
    fn cd_rejects_missing_dir() {
        let fs = FileSystem::new();
        let mut manager = FileManager::new();
        assert_eq!(manager.cd(&fs, "/missing"), Err(FsError::NotFound));
    }

    #[test]
    fn cd_parent_directory() {
        let mut fs = FileSystem::new();
        fs.mkdir("/home").unwrap();
        fs.mkdir("/home/user").unwrap();
        let mut manager = FileManager::new();
        manager.cd(&fs, "/home/user").unwrap();
        manager.cd(&fs, "..").unwrap();
        assert_eq!(manager.pwd(), "/home");
        manager.cd(&fs, "..").unwrap();
        assert_eq!(manager.pwd(), "/");
    }

    #[test]
    fn cd_rejects_empty_path() {
        let fs = FileSystem::new();
        let mut manager = FileManager::new();
        assert_eq!(manager.cd(&fs, ""), Err(FsError::InvalidPath));
    }

    #[test]
    fn cat_rejects_invalid_utf8() {
        let mut fs = FileSystem::new();
        fs.mkdir("/data").unwrap();
        fs.write_file("/data/blob", &[0xFF]).unwrap();
        let manager = FileManager::new();
        assert_eq!(manager.cat(&fs, "/data/blob"), Err(FsError::InvalidUtf8));
    }

    #[test]
    fn cat_rejects_missing_file() {
        let fs = FileSystem::new();
        let manager = FileManager::new();
        assert_eq!(manager.cat(&fs, "/missing"), Err(FsError::NotFound));
    }

    #[test]
    fn cat_rejects_empty_path() {
        let fs = FileSystem::new();
        let manager = FileManager::new();
        assert_eq!(manager.cat(&fs, ""), Err(FsError::InvalidPath));
    }

    #[test]
    fn write_rejects_empty_path() {
        let mut fs = FileSystem::new();
        let manager = FileManager::new();
        assert_eq!(manager.write(&mut fs, "", "hi"), Err(FsError::InvalidPath));
    }

    #[test]
    fn mkdir_rejects_empty_path() {
        let mut fs = FileSystem::new();
        let manager = FileManager::new();
        assert_eq!(manager.mkdir(&mut fs, ""), Err(FsError::InvalidPath));
    }

    #[test]
    fn rm_rejects_empty_path() {
        let mut fs = FileSystem::new();
        let manager = FileManager::new();
        assert_eq!(manager.rm(&mut fs, ""), Err(FsError::InvalidPath));
    }

    #[test]
    fn mkdir_and_remove_relative_paths() {
        let mut fs = FileSystem::new();
        fs.mkdir("/home").unwrap();
        let mut manager = FileManager::new();
        manager.cd(&fs, "/home").unwrap();
        manager.mkdir(&mut fs, "docs").unwrap();
        let list = manager.ls(&fs).unwrap();
        assert_eq!(list, vec!["docs".to_string()]);
        manager.rm(&mut fs, "docs").unwrap();
        assert!(manager.ls(&fs).unwrap().is_empty());
    }

    #[test]
    fn ls_path_resolves_relative_paths() {
        let mut fs = FileSystem::new();
        fs.mkdir("/home").unwrap();
        fs.mkdir("/home/docs").unwrap();
        let mut manager = FileManager::new();
        manager.cd(&fs, "/home").unwrap();
        let list = manager.ls_path(&fs, "docs").unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn ls_path_rejects_empty() {
        let fs = FileSystem::new();
        let manager = FileManager::new();
        assert_eq!(manager.ls_path(&fs, ""), Err(FsError::InvalidPath));
    }

    #[test]
    fn file_system_dependency_error_paths() {
        let mut fs = FileSystem::new();

        assert_eq!(fs.mkdir("foo//bar"), Err(FsError::InvalidPath));
        assert_eq!(fs.write_file("foo//bar", b"x"), Err(FsError::InvalidPath));
        assert_eq!(fs.read_file("foo//bar"), Err(FsError::InvalidPath));
        assert_eq!(fs.list_dir("foo//bar"), Err(FsError::InvalidPath));
        assert_eq!(fs.remove("foo//bar"), Err(FsError::InvalidPath));

        fs.mkdir("/tmp").unwrap();
        assert_eq!(fs.mkdir("/tmp"), Err(FsError::AlreadyExists));

        fs.write_file("/tmp/file", b"a").unwrap();
        fs.write_file("/tmp/file", b"b").unwrap();
        assert_eq!(fs.write_file("/tmp", b"x"), Err(FsError::IsDir));

        assert_eq!(fs.read_file("/"), Err(FsError::IsDir));
        assert_eq!(fs.list_dir("/").unwrap(), vec!["tmp".to_string()]);
        assert_eq!(fs.list_dir("/tmp/file"), Err(FsError::NotDir));
        assert_eq!(fs.list_dir("/missing"), Err(FsError::NotFound));

        assert_eq!(fs.remove("/missing"), Err(FsError::NotFound));
        assert_eq!(fs.remove("/tmp"), Err(FsError::NotEmpty));

        fs.remove("/tmp/file").unwrap();
        fs.remove("/tmp").unwrap();

        fs.write_file("/file", b"x").unwrap();
        assert_eq!(fs.mkdir("/file/sub"), Err(FsError::NotDir));
    }
}
