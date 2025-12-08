#[cfg(test)]
mod filesystem_tests;

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    pub size: u64,
    pub checksum: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub metadata: Option<FileMetadata>,
}

#[derive(Debug, Clone)]
struct FileRecord {
    data: Vec<u8>,
    checksum: [u8; 32],
    size: u64,
}

#[derive(Debug, Clone, Default)]
struct Directory {
    entries: HashMap<String, Node>,
}

#[derive(Debug, Clone)]
enum Node {
    File(FileRecord),
    Directory(Directory),
}

#[derive(Debug, Default, Clone)]
pub struct ChecksumFs {
    root: Directory,
}

impl ChecksumFs {
    pub fn new() -> Self {
        Self {
            root: Directory::default(),
        }
    }

    pub fn mkdir_all(&mut self, path: &Path) -> Result<()> {
        let components = normalize_components(path)?;
        self.ensure_dir_from_components(&components).map(|_| ())
    }

    pub fn write_file(&mut self, path: &Path, data: &[u8]) -> Result<FileMetadata> {
        let mut components = normalize_components(path)?;
        let file_name = components.pop().ok_or_else(|| anyhow!("path is empty"))?;
        let dir = self.ensure_dir_from_components(&components)?;

        let checksum = compute_checksum(data);
        let record = FileRecord {
            data: data.to_vec(),
            checksum,
            size: data.len() as u64,
        };
        dir.entries.insert(file_name.clone(), Node::File(record));
        Ok(FileMetadata {
            size: data.len() as u64,
            checksum,
        })
    }

    pub fn read_file(&self, path: &Path) -> Result<Vec<u8>> {
        match self.get_node(path)? {
            Node::File(file) => Ok(file.data.clone()),
            Node::Directory(_) => bail!("{} is a directory", path.display()),
        }
    }

    pub fn verify(&self, path: &Path) -> Result<bool> {
        match self.get_node(path)? {
            Node::File(file) => Ok(file.checksum == compute_checksum(&file.data)),
            Node::Directory(_) => bail!("{} is a directory", path.display()),
        }
    }

    pub fn metadata(&self, path: &Path) -> Result<DirEntry> {
        if path.components().count() == 0 {
            return Ok(DirEntry {
                name: String::from("/"),
                is_dir: true,
                metadata: None,
            });
        }

        let components = normalize_components(path)?;
        let (name, node) = self.node_from_components(&components)?;
        match node {
            Node::Directory(_) => Ok(DirEntry {
                name,
                is_dir: true,
                metadata: None,
            }),
            Node::File(file) => Ok(DirEntry {
                name,
                is_dir: false,
                metadata: Some(FileMetadata {
                    size: file.size,
                    checksum: file.checksum,
                }),
            }),
        }
    }

    pub fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>> {
        let dir = self.resolve_dir(path)?;
        Ok(dir
            .entries
            .iter()
            .map(|(name, node)| match node {
                Node::Directory(_) => DirEntry {
                    name: name.clone(),
                    is_dir: true,
                    metadata: None,
                },
                Node::File(file) => DirEntry {
                    name: name.clone(),
                    is_dir: false,
                    metadata: Some(FileMetadata {
                        size: file.size,
                        checksum: file.checksum,
                    }),
                },
            })
            .collect())
    }

    fn resolve_dir(&self, path: &Path) -> Result<&Directory> {
        let components = normalize_components(path)?;
        self.resolve_dir_from_components(&components)
    }

    fn ensure_dir_from_components(&mut self, components: &[String]) -> Result<&mut Directory> {
        let mut current = &mut self.root;
        for name in components {
            let node = current
                .entries
                .entry(name.clone())
                .or_insert_with(|| Node::Directory(Directory::default()));

            current = match node {
                Node::Directory(dir) => dir,
                Node::File(_) => bail!("{name} is a file"),
            };
        }
        Ok(current)
    }

    fn get_node(&self, path: &Path) -> Result<&Node> {
        let components = normalize_components(path)?;
        self.node_from_components(&components).map(|(_, node)| node)
    }

    fn resolve_dir_from_components(&self, components: &[String]) -> Result<&Directory> {
        let mut current = &self.root;
        for name in components {
            current = match current.entries.get(name) {
                Some(Node::Directory(dir)) => dir,
                Some(Node::File(_)) => bail!("{name} is a file"),
                None => bail!("missing directory {name}"),
            };
        }
        Ok(current)
    }

    fn node_from_components(&self, components: &[String]) -> Result<(String, &Node)> {
        let (name, parent_components) = components
            .split_last()
            .ok_or_else(|| anyhow!("path is empty"))?;
        let dir = self.resolve_dir_from_components(parent_components)?;
        let name = name.to_owned();
        let node = dir
            .entries
            .get(&name)
            .ok_or_else(|| anyhow!("{name} not found"))?;
        Ok((name, node))
    }
}

fn normalize_components(path: &Path) -> Result<Vec<String>> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::ParentDir => bail!("parent directory references are not supported"),
            Component::Normal(name) => components.push(name_to_string(name)?),
            Component::Prefix(prefix) => {
                let path: PathBuf = prefix.as_os_str().into();
                components.push(path.to_string_lossy().to_string());
            }
        }
    }
    Ok(components)
}

fn name_to_string(name: &std::ffi::OsStr) -> Result<String> {
    name.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("non utf-8 path component"))
}

fn compute_checksum(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}