//! Repository management for Forjj.
//!
//! This module provides high-level repository operations, wrapping jj-lib's
//! storage backend to provide a clean API for the rest of Forjj.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

/// Repository information.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub name: String,
    pub path: PathBuf,
    pub backend_type: BackendType,
}

/// Supported backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    /// Native jj backend (SimpleBackend)
    Native,
    /// Git backend
    Git,
}

impl BackendType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BackendType::Native => "simple",
            BackendType::Git => "git",
        }
    }
}

/// Repository storage configuration.
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Root directory for all repositories
    pub repos_root: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            repos_root: PathBuf::from("/var/forjj/repos"),
        }
    }
}

/// Repository manager for creating and accessing repositories.
pub struct RepositoryManager {
    config: StorageConfig,
}

impl RepositoryManager {
    /// Create a new repository manager with the given configuration.
    pub fn new(config: StorageConfig) -> Self {
        Self { config }
    }

    /// Get the path to a repository.
    pub fn repo_path(&self, owner: &str, name: &str) -> PathBuf {
        self.config.repos_root.join(owner).join(name)
    }

    /// Check if a repository exists.
    pub fn repo_exists(&self, owner: &str, name: &str) -> bool {
        let path = self.repo_path(owner, name);
        path.join(".jj").exists()
    }

    /// List all repositories for an owner.
    pub fn list_repos(&self, owner: &str) -> Result<Vec<RepoInfo>> {
        let owner_path = self.config.repos_root.join(owner);
        if !owner_path.exists() {
            return Ok(Vec::new());
        }

        let mut repos = Vec::new();
        for entry in std::fs::read_dir(&owner_path)
            .with_context(|| format!("failed to read directory: {}", owner_path.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.join(".jj").exists() {
                let name = entry.file_name().to_string_lossy().to_string();
                let backend_type = self.detect_backend_type(&path)?;
                repos.push(RepoInfo {
                    name,
                    path,
                    backend_type,
                });
            }
        }

        Ok(repos)
    }

    /// Detect the backend type of a repository.
    fn detect_backend_type(&self, repo_path: &Path) -> Result<BackendType> {
        let type_file = repo_path.join(".jj/repo/store/type");
        if type_file.exists() {
            let content = std::fs::read_to_string(&type_file)
                .with_context(|| format!("failed to read: {}", type_file.display()))?;
            let backend = content.trim();
            match backend {
                "simple" => Ok(BackendType::Native),
                "git" => Ok(BackendType::Git),
                _ => {
                    info!("unknown backend type: {}, assuming git", backend);
                    Ok(BackendType::Git)
                }
            }
        } else {
            // Default to git if type file doesn't exist
            Ok(BackendType::Git)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repo_path() {
        let config = StorageConfig {
            repos_root: PathBuf::from("/data/repos"),
        };
        let manager = RepositoryManager::new(config);
        assert_eq!(
            manager.repo_path("alice", "my-project"),
            PathBuf::from("/data/repos/alice/my-project")
        );
    }

    #[test]
    fn test_backend_type_as_str() {
        assert_eq!(BackendType::Native.as_str(), "simple");
        assert_eq!(BackendType::Git.as_str(), "git");
    }
}
