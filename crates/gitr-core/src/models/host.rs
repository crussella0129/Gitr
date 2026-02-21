use serde::{Deserialize, Serialize};
use url::Url;
use uuid::Uuid;

/// Unique identifier for a host.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HostId(pub Uuid);

impl HostId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for HostId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The kind of git hosting service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    GitHub,
    GitLab,
    Gitea,
    Bitbucket,
    AzureDevOps,
}

impl std::fmt::Display for HostKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostKind::GitHub => write!(f, "github"),
            HostKind::GitLab => write!(f, "gitlab"),
            HostKind::Gitea => write!(f, "gitea"),
            HostKind::Bitbucket => write!(f, "bitbucket"),
            HostKind::AzureDevOps => write!(f, "azure_devops"),
        }
    }
}

impl std::str::FromStr for HostKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github" => Ok(HostKind::GitHub),
            "gitlab" => Ok(HostKind::GitLab),
            "gitea" => Ok(HostKind::Gitea),
            "bitbucket" => Ok(HostKind::Bitbucket),
            "azure_devops" | "azure-devops" | "azuredevops" => Ok(HostKind::AzureDevOps),
            _ => Err(format!("unknown host kind: {s}")),
        }
    }
}

impl HostKind {
    /// Default API URL for this host kind.
    pub fn default_api_url(&self) -> Url {
        match self {
            HostKind::GitHub => Url::parse("https://api.github.com").unwrap(),
            HostKind::GitLab => Url::parse("https://gitlab.com/api/v4").unwrap(),
            HostKind::Gitea => Url::parse("https://gitea.com/api/v1").unwrap(),
            HostKind::Bitbucket => Url::parse("https://api.bitbucket.org/2.0").unwrap(),
            HostKind::AzureDevOps => Url::parse("https://dev.azure.com").unwrap(),
        }
    }
}

/// A registered git hosting service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: HostId,
    pub label: String,
    pub kind: HostKind,
    pub api_url: Url,
    pub username: String,
    /// Key used to look up the token in the OS keychain.
    pub credential_key: String,
}

impl Host {
    pub fn new(label: String, kind: HostKind, username: String) -> Self {
        let api_url = kind.default_api_url();
        let credential_key = format!("gitr:{label}");
        Self {
            id: HostId::new(),
            label,
            kind,
            api_url,
            username,
            credential_key,
        }
    }
}
