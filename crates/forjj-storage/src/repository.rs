//! Repository management for Forjj.
//!
//! This module provides high-level repository operations, wrapping jj-lib's
//! storage backend to provide a clean API for the rest of Forjj.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use jj_lib::backend::CommitId;
use jj_lib::commit::Commit;
use jj_lib::config::StackedConfig;
use jj_lib::repo::{ReadonlyRepo, Repo, StoreFactories};
use jj_lib::settings::UserSettings;
use jj_lib::workspace::{Workspace, default_working_copy_factories};
use tracing::{debug, info};

/// Repository information.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub name: String,
    pub owner: String,
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

/// A handle to an opened jj repository.
pub struct Repository {
    #[allow(dead_code)]
    workspace: Workspace,
    repo: Arc<ReadonlyRepo>,
    info: RepoInfo,
}

impl Repository {
    /// Get the repository information.
    pub fn info(&self) -> &RepoInfo {
        &self.info
    }

    /// Get the underlying jj-lib repository.
    pub fn repo(&self) -> &Arc<ReadonlyRepo> {
        &self.repo
    }

    /// Get a commit by its ID.
    pub fn get_commit(&self, id: &CommitId) -> Result<Commit> {
        self.repo
            .store()
            .get_commit(id)
            .context("failed to get commit")
    }

    /// Get the root commit (empty commit that all commits descend from).
    pub fn root_commit(&self) -> Commit {
        self.repo.store().root_commit()
    }

    /// Get all visible heads (commits with no children in the view).
    pub fn heads(&self) -> Vec<CommitId> {
        self.repo.view().heads().iter().cloned().collect()
    }

    /// Get all bookmarks (named refs).
    pub fn bookmarks(&self) -> Vec<(String, CommitId)> {
        self.repo
            .view()
            .bookmarks()
            .map(|(name, target)| {
                let commit_id = target.local_target.as_normal().cloned();
                (
                    name.as_str().to_string(),
                    commit_id.unwrap_or_else(|| self.root_commit().id().clone()),
                )
            })
            .collect()
    }

    /// Get the current operation ID.
    pub fn operation_id(&self) -> &jj_lib::op_store::OperationId {
        self.repo.op_id()
    }
}

/// Repository manager for creating and accessing repositories.
pub struct RepositoryManager {
    config: StorageConfig,
    user_settings: UserSettings,
    store_factories: StoreFactories,
}

impl RepositoryManager {
    /// Create a new repository manager with the given configuration.
    pub fn new(config: StorageConfig) -> Result<Self> {
        let jj_config = StackedConfig::with_defaults();
        let user_settings =
            UserSettings::from_config(jj_config).context("failed to create user settings")?;
        let store_factories = StoreFactories::default();

        Ok(Self {
            config,
            user_settings,
            store_factories,
        })
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

    /// Create a new repository with the native jj backend.
    pub fn create_repo(&self, owner: &str, name: &str) -> Result<Repository> {
        let repo_path = self.repo_path(owner, name);

        if repo_path.exists() {
            bail!("repository already exists: {}/{}", owner, name);
        }

        // Create parent directories
        std::fs::create_dir_all(&repo_path)
            .with_context(|| format!("failed to create directory: {}", repo_path.display()))?;

        info!("creating repository at {}", repo_path.display());

        // Initialize with native (simple) backend
        let (workspace, repo) = Workspace::init_simple(&self.user_settings, &repo_path)
            .with_context(|| format!("failed to init repository at {}", repo_path.display()))?;

        debug!("repository created with backend: simple");

        let info = RepoInfo {
            name: name.to_string(),
            owner: owner.to_string(),
            path: repo_path,
            backend_type: BackendType::Native,
        };

        Ok(Repository {
            workspace,
            repo,
            info,
        })
    }

    /// Open an existing repository.
    pub fn open_repo(&self, owner: &str, name: &str) -> Result<Repository> {
        let repo_path = self.repo_path(owner, name);

        if !repo_path.join(".jj").exists() {
            bail!("repository does not exist: {}/{}", owner, name);
        }

        debug!("opening repository at {}", repo_path.display());

        let workspace = Workspace::load(
            &self.user_settings,
            &repo_path,
            &self.store_factories,
            &default_working_copy_factories(),
        )
        .with_context(|| format!("failed to load workspace at {}", repo_path.display()))?;

        let repo = workspace
            .repo_loader()
            .load_at_head()
            .context("failed to load repository at head")?;

        let backend_type = self.detect_backend_type(&repo_path)?;

        let info = RepoInfo {
            name: name.to_string(),
            owner: owner.to_string(),
            path: repo_path,
            backend_type,
        };

        Ok(Repository {
            workspace,
            repo,
            info,
        })
    }

    /// Delete a repository.
    pub fn delete_repo(&self, owner: &str, name: &str) -> Result<()> {
        let repo_path = self.repo_path(owner, name);

        if !repo_path.exists() {
            bail!("repository does not exist: {}/{}", owner, name);
        }

        info!("deleting repository at {}", repo_path.display());

        std::fs::remove_dir_all(&repo_path)
            .with_context(|| format!("failed to delete: {}", repo_path.display()))?;

        Ok(())
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
                    owner: owner.to_string(),
                    path,
                    backend_type,
                });
            }
        }

        Ok(repos)
    }

    /// List all owners.
    pub fn list_owners(&self) -> Result<Vec<String>> {
        if !self.config.repos_root.exists() {
            return Ok(Vec::new());
        }

        let mut owners = Vec::new();
        for entry in std::fs::read_dir(&self.config.repos_root)
            .with_context(|| format!("failed to read: {}", self.config.repos_root.display()))?
        {
            let entry = entry?;
            if entry.path().is_dir() {
                owners.push(entry.file_name().to_string_lossy().to_string());
            }
        }

        Ok(owners)
    }

    /// Detect the backend type of a repository.
    fn detect_backend_type(&self, repo_path: &Path) -> Result<BackendType> {
        let type_file = repo_path.join(".jj/repo/store/type");
        if type_file.exists() {
            let content = std::fs::read_to_string(&type_file)
                .with_context(|| format!("failed to read: {}", type_file.display()))?;
            let backend = content.trim();
            match backend.to_lowercase().as_str() {
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
    use jj_lib::object_id::ObjectId as _;
    use tempfile::TempDir;

    #[test]
    fn test_repo_path() {
        let config = StorageConfig {
            repos_root: PathBuf::from("/data/repos"),
        };
        let manager = RepositoryManager::new(config).unwrap();
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

    #[test]
    fn test_create_and_open_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            repos_root: temp_dir.path().to_path_buf(),
        };
        let manager = RepositoryManager::new(config).unwrap();

        // Create a new repository
        let repo = manager.create_repo("alice", "test-repo").unwrap();
        assert_eq!(repo.info().name, "test-repo");
        assert_eq!(repo.info().owner, "alice");
        assert_eq!(repo.info().backend_type, BackendType::Native);

        // Verify it exists
        assert!(manager.repo_exists("alice", "test-repo"));

        // Open the repository
        let repo2 = manager.open_repo("alice", "test-repo").unwrap();
        assert_eq!(repo2.info().name, "test-repo");
        assert_eq!(repo2.info().backend_type, BackendType::Native);

        // List repositories
        let repos = manager.list_repos("alice").unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "test-repo");
    }

    #[test]
    fn test_delete_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            repos_root: temp_dir.path().to_path_buf(),
        };
        let manager = RepositoryManager::new(config).unwrap();

        // Create and then delete
        manager.create_repo("bob", "to-delete").unwrap();
        assert!(manager.repo_exists("bob", "to-delete"));

        manager.delete_repo("bob", "to-delete").unwrap();
        assert!(!manager.repo_exists("bob", "to-delete"));
    }

    #[test]
    fn test_read_commits() {
        let temp_dir = TempDir::new().unwrap();
        let config = StorageConfig {
            repos_root: temp_dir.path().to_path_buf(),
        };
        let manager = RepositoryManager::new(config).unwrap();

        // Create a new repository
        let repo = manager.create_repo("alice", "commits-test").unwrap();

        // Every jj repo has a root commit
        let root = repo.root_commit();
        assert!(root.parent_ids().is_empty());

        // Get the root commit by ID
        let root_id = root.id().clone();
        let fetched = repo.get_commit(&root_id).unwrap();
        assert_eq!(fetched.id(), root.id());

        // New repo should have heads
        let heads = repo.heads();
        assert!(!heads.is_empty());

        // Get operation ID
        let op_id = repo.operation_id();
        assert!(!op_id.hex().is_empty());
    }
}
