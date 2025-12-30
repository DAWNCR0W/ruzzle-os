#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Errors returned by the in-memory filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    NotDir,
    IsDir,
    AlreadyExists,
    InvalidPath,
    NotEmpty,
    InvalidUtf8,
}

/// Filesystem usage statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FsStats {
    pub files: usize,
    pub dirs: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    File(Vec<u8>),
    Dir(BTreeMap<String, Node>),
}

/// In-memory filesystem used by the fs-service module.
#[derive(Debug, Default, Clone)]
pub struct FileSystem {
    root: BTreeMap<String, Node>,
}

impl FileSystem {
    /// Creates an empty filesystem.
    pub fn new() -> Self {
        Self {
            root: BTreeMap::new(),
        }
    }

    /// Creates a directory at the provided path.
    pub fn mkdir(&mut self, path: &str) -> Result<(), FsError> {
        let parts = split_path(path)?;
        if parts.is_empty() {
            return Err(FsError::InvalidPath);
        }
        let (parent, name) = self.walk_parent_mut(&parts)?;
        if parent.contains_key(&name) {
            return Err(FsError::AlreadyExists);
        }
        parent.insert(name, Node::Dir(BTreeMap::new()));
        Ok(())
    }

    /// Writes a file, creating it if missing.
    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FsError> {
        let parts = split_path(path)?;
        if parts.is_empty() {
            return Err(FsError::InvalidPath);
        }
        let (parent, name) = self.walk_parent_mut(&parts)?;
        match parent.get_mut(&name) {
            Some(Node::Dir(_)) => Err(FsError::IsDir),
            Some(Node::File(existing)) => {
                existing.clear();
                existing.extend_from_slice(data);
                Ok(())
            }
            None => {
                parent.insert(name, Node::File(data.to_vec()));
                Ok(())
            }
        }
    }

    /// Reads a file and returns its bytes.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, FsError> {
        let parts = split_path(path)?;
        if parts.is_empty() {
            return Err(FsError::IsDir);
        }
        match self.walk_node(&parts)? {
            Node::File(data) => Ok(data.clone()),
            Node::Dir(_) => Err(FsError::IsDir),
        }
    }

    /// Lists a directory, returning entries sorted by name.
    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let parts = split_path(path)?;
        let node = if parts.is_empty() {
            None
        } else {
            Some(self.walk_node(&parts)?)
        };
        let dir = match node {
            None => &self.root,
            Some(Node::Dir(children)) => children,
            Some(Node::File(_)) => return Err(FsError::NotDir),
        };
        Ok(dir.keys().cloned().collect())
    }

    /// Returns usage stats for the entire filesystem.
    pub fn stats(&self) -> FsStats {
        let mut stats = FsStats {
            files: 0,
            dirs: 0,
            bytes: 0,
        };
        count_dir(&self.root, &mut stats);
        stats
    }

    /// Returns usage stats for a specific path.
    pub fn stats_for(&self, path: &str) -> Result<FsStats, FsError> {
        let parts = split_path(path)?;
        if parts.is_empty() {
            return Ok(self.stats());
        }
        let node = self.walk_node(&parts)?;
        let mut stats = FsStats {
            files: 0,
            dirs: 0,
            bytes: 0,
        };
        match node {
            Node::File(data) => {
                stats.files = 1;
                stats.bytes = data.len();
            }
            Node::Dir(children) => {
                count_dir(children, &mut stats);
            }
        }
        Ok(stats)
    }

    /// Returns the total byte size for a file or directory tree.
    pub fn size_of(&self, path: &str) -> Result<usize, FsError> {
        Ok(self.stats_for(path)?.bytes)
    }

    /// Removes a file or an empty directory.
    pub fn remove(&mut self, path: &str) -> Result<(), FsError> {
        let parts = split_path(path)?;
        if parts.is_empty() {
            return Err(FsError::InvalidPath);
        }
        let (parent, name) = self.walk_parent_mut(&parts)?;
        match parent.get(&name) {
            None => Err(FsError::NotFound),
            Some(Node::Dir(children)) if !children.is_empty() => Err(FsError::NotEmpty),
            _ => {
                parent.remove(&name);
                Ok(())
            }
        }
    }

    fn walk_node<'a>(&'a self, parts: &[&str]) -> Result<&'a Node, FsError> {
        let mut current = &self.root;
        for (index, segment) in parts.iter().enumerate() {
            let node = current.get(*segment).ok_or(FsError::NotFound)?;
            if index == parts.len() - 1 {
                return Ok(node);
            }
            match node {
                Node::Dir(children) => current = children,
                Node::File(_) => return Err(FsError::NotDir),
            }
        }
        Err(FsError::NotFound)
    }

    fn walk_parent_mut(
        &mut self,
        parts: &[&str],
    ) -> Result<(&mut BTreeMap<String, Node>, String), FsError> {
        let (path, name) = parts.split_at(parts.len() - 1);
        let mut current = &mut self.root;
        for segment in path {
            let node = current.get_mut(*segment).ok_or(FsError::NotFound)?;
            match node {
                Node::Dir(children) => current = children,
                Node::File(_) => return Err(FsError::NotDir),
            }
        }
        Ok((current, name[0].to_string()))
    }
}

fn split_path(path: &str) -> Result<Vec<&str>, FsError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(FsError::InvalidPath);
    }
    if trimmed != "/" && trimmed.ends_with('/') {
        return Err(FsError::InvalidPath);
    }
    if trimmed.contains("//") {
        return Err(FsError::InvalidPath);
    }
    if trimmed == "/" {
        return Ok(Vec::new());
    }
    let mut parts = Vec::new();
    for segment in trimmed.split('/') {
        if segment.is_empty() {
            continue;
        }
        if segment == "." || segment == ".." {
            return Err(FsError::InvalidPath);
        }
        parts.push(segment);
    }
    Ok(parts)
}

fn count_dir(children: &BTreeMap<String, Node>, stats: &mut FsStats) {
    stats.dirs += 1;
    for node in children.values() {
        match node {
            Node::File(data) => {
                stats.files += 1;
                stats.bytes += data.len();
            }
            Node::Dir(grandchildren) => count_dir(grandchildren, stats),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_path_accepts_root() {
        let parts = split_path("/").unwrap();
        assert!(parts.is_empty());
    }

    #[test]
    fn split_path_rejects_invalid() {
        assert_eq!(split_path(""), Err(FsError::InvalidPath));
        assert_eq!(split_path("/foo/"), Err(FsError::InvalidPath));
        assert_eq!(split_path("foo//bar"), Err(FsError::InvalidPath));
        assert_eq!(split_path("./foo"), Err(FsError::InvalidPath));
        assert_eq!(split_path("foo/.."), Err(FsError::InvalidPath));
    }

    #[test]
    fn mkdir_and_list() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.mkdir("/var").unwrap();
        let list = fs.list_dir("/").unwrap();
        assert_eq!(list, vec!["etc".to_string(), "var".to_string()]);
    }

    #[test]
    fn list_dir_on_subdirectory() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"127.0.0.1").unwrap();
        fs.mkdir("/etc/conf").unwrap();
        let list = fs.list_dir("/etc").unwrap();
        assert_eq!(list, vec!["conf".to_string(), "hosts".to_string()]);
    }

    #[test]
    fn mkdir_rejects_existing() {
        let mut fs = FileSystem::new();
        fs.mkdir("/tmp").unwrap();
        assert_eq!(fs.mkdir("/tmp"), Err(FsError::AlreadyExists));
    }

    #[test]
    fn mkdir_rejects_root_path() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.mkdir("/"), Err(FsError::InvalidPath));
    }

    #[test]
    fn mkdir_rejects_invalid_path_syntax() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.mkdir("foo//bar"), Err(FsError::InvalidPath));
    }

    #[test]
    fn mkdir_requires_parent() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.mkdir("/a/b"), Err(FsError::NotFound));
    }

    #[test]
    fn stats_empty_fs_counts_root_dir() {
        let fs = FileSystem::new();
        let stats = fs.stats();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.dirs, 1);
        assert_eq!(stats.bytes, 0);
    }

    #[test]
    fn stats_counts_nested_entries() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.mkdir("/var").unwrap();
        fs.write_file("/etc/hosts", b"abc").unwrap();
        fs.write_file("/var/log", b"xy").unwrap();
        let stats = fs.stats();
        assert_eq!(stats.files, 2);
        assert_eq!(stats.dirs, 3);
        assert_eq!(stats.bytes, 5);
    }

    #[test]
    fn stats_for_file_and_dir() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"abc").unwrap();
        let file_stats = fs.stats_for("/etc/hosts").unwrap();
        assert_eq!(file_stats.files, 1);
        assert_eq!(file_stats.dirs, 0);
        assert_eq!(file_stats.bytes, 3);
        let dir_stats = fs.stats_for("/etc").unwrap();
        assert_eq!(dir_stats.files, 1);
        assert_eq!(dir_stats.dirs, 1);
        assert_eq!(dir_stats.bytes, 3);
    }

    #[test]
    fn stats_for_rejects_invalid_path_syntax() {
        let fs = FileSystem::new();
        assert_eq!(fs.stats_for("bad//path"), Err(FsError::InvalidPath));
    }

    #[test]
    fn size_of_handles_root_and_missing() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"abc").unwrap();
        assert_eq!(fs.size_of("/").unwrap(), 3);
        assert_eq!(fs.size_of("/etc").unwrap(), 3);
        assert_eq!(fs.size_of("/etc/hosts").unwrap(), 3);
        assert_eq!(fs.size_of("/missing"), Err(FsError::NotFound));
    }

    #[test]
    fn write_and_read_file() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"127.0.0.1").unwrap();
        let data = fs.read_file("/etc/hosts").unwrap();
        assert_eq!(data, b"127.0.0.1".to_vec());
    }

    #[test]
    fn write_overwrites_file() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"a").unwrap();
        fs.write_file("/etc/hosts", b"b").unwrap();
        let data = fs.read_file("/etc/hosts").unwrap();
        assert_eq!(data, b"b".to_vec());
    }

    #[test]
    fn write_rejects_dir_target() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        assert_eq!(fs.write_file("/etc", b"x"), Err(FsError::IsDir));
    }

    #[test]
    fn write_rejects_root_path() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.write_file("/", b"x"), Err(FsError::InvalidPath));
    }

    #[test]
    fn write_rejects_invalid_path_syntax() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.write_file("foo//bar", b"x"), Err(FsError::InvalidPath));
    }

    #[test]
    fn write_rejects_missing_parent() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.write_file("/missing/file", b"x"), Err(FsError::NotFound));
    }

    #[test]
    fn read_rejects_missing() {
        let fs = FileSystem::new();
        assert_eq!(fs.read_file("/missing"), Err(FsError::NotFound));
    }

    #[test]
    fn read_rejects_invalid_path_syntax() {
        let fs = FileSystem::new();
        assert_eq!(fs.read_file("foo//bar"), Err(FsError::InvalidPath));
    }

    #[test]
    fn read_rejects_directories() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        assert_eq!(fs.read_file("/"), Err(FsError::IsDir));
        assert_eq!(fs.read_file("/etc"), Err(FsError::IsDir));
    }

    #[test]
    fn list_dir_rejects_file() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"x").unwrap();
        assert_eq!(fs.list_dir("/etc/hosts"), Err(FsError::NotDir));
    }

    #[test]
    fn list_dir_rejects_missing() {
        let fs = FileSystem::new();
        assert_eq!(fs.list_dir("/missing"), Err(FsError::NotFound));
    }

    #[test]
    fn list_dir_rejects_invalid_path_syntax() {
        let fs = FileSystem::new();
        assert_eq!(fs.list_dir("foo//bar"), Err(FsError::InvalidPath));
    }

    #[test]
    fn remove_file_and_empty_dir() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"x").unwrap();
        fs.remove("/etc/hosts").unwrap();
        fs.remove("/etc").unwrap();
        assert_eq!(fs.list_dir("/"), Ok(Vec::new()));
    }

    #[test]
    fn remove_rejects_non_empty_dir() {
        let mut fs = FileSystem::new();
        fs.mkdir("/etc").unwrap();
        fs.write_file("/etc/hosts", b"x").unwrap();
        assert_eq!(fs.remove("/etc"), Err(FsError::NotEmpty));
    }

    #[test]
    fn remove_rejects_missing() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.remove("/nope"), Err(FsError::NotFound));
    }

    #[test]
    fn remove_rejects_root_path() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.remove("/"), Err(FsError::InvalidPath));
    }

    #[test]
    fn remove_rejects_invalid_path_syntax() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.remove("foo//bar"), Err(FsError::InvalidPath));
    }

    #[test]
    fn remove_rejects_missing_parent() {
        let mut fs = FileSystem::new();
        assert_eq!(fs.remove("/missing/file"), Err(FsError::NotFound));
    }

    #[test]
    fn read_rejects_intermediate_file() {
        let mut fs = FileSystem::new();
        fs.write_file("/etc", b"x").unwrap();
        assert_eq!(fs.read_file("/etc/hosts"), Err(FsError::NotDir));
    }

    #[test]
    fn mkdir_rejects_intermediate_file() {
        let mut fs = FileSystem::new();
        fs.write_file("/etc", b"x").unwrap();
        assert_eq!(fs.mkdir("/etc/hosts"), Err(FsError::NotDir));
    }

    #[test]
    fn list_root_on_empty_fs() {
        let fs = FileSystem::new();
        let list = fs.list_dir("/").unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn walk_node_rejects_empty_parts() {
        let fs = FileSystem::new();
        assert_eq!(fs.walk_node(&[]), Err(FsError::NotFound));
    }
}
