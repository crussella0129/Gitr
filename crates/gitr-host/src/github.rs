use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::Deserialize;

use gitr_core::error::GitrError;
use gitr_core::models::host::HostKind;

use crate::{ForkSyncStatus, HostProvider, RateLimitInfo, RemoteBranch, RemoteRepo};

pub struct GitHubProvider {
    client: reqwest::Client,
    api_url: url::Url,
    #[allow(dead_code)]
    username: String,
}

impl GitHubProvider {
    pub fn new(api_url: url::Url, token: String, username: String) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        if let Ok(val) = HeaderValue::from_str(&format!("Bearer {token}")) {
            headers.insert(header::AUTHORIZATION, val);
        }
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("gitr/0.1.0"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            api_url,
            username,
        }
    }

    fn url(&self, path: &str) -> String {
        let base = self.api_url.as_str().trim_end_matches('/');
        format!("{base}{path}")
    }

    async fn paginated_get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        per_page: u32,
    ) -> Result<Vec<T>, GitrError> {
        let mut all = Vec::new();
        let mut page = 1u32;

        loop {
            let url = format!("{}?per_page={per_page}&page={page}", self.url(path));
            let resp = self
                .client
                .get(&url)
                .send()
                .await
                .map_err(|e| GitrError::ApiError {
                    status: 0,
                    message: e.to_string(),
                })?;

            let status = resp.status().as_u16();
            if status == 403 || status == 429 {
                return Err(GitrError::RateLimited {
                    host: "github.com".to_string(),
                    retry_after_secs: 60,
                });
            }
            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(GitrError::ApiError {
                    status,
                    message: body,
                });
            }

            let items: Vec<T> = resp.json().await.map_err(|e| GitrError::ApiError {
                status: 0,
                message: format!("JSON parse error: {e}"),
            })?;

            let count = items.len();
            all.extend(items);

            if count < per_page as usize {
                break;
            }
            page += 1;
        }

        Ok(all)
    }
}

#[derive(Deserialize)]
struct GhRepo {
    full_name: String,
    name: String,
    owner: GhOwner,
    clone_url: String,
    ssh_url: String,
    default_branch: Option<String>,
    fork: bool,
    parent: Option<Box<GhRepo>>,
    description: Option<String>,
    private: bool,
    archived: bool,
    updated_at: Option<String>,
}

#[derive(Deserialize)]
struct GhOwner {
    login: String,
}

#[derive(Deserialize)]
struct GhBranch {
    name: String,
    commit: GhCommitRef,
}

#[derive(Deserialize)]
struct GhCommitRef {
    sha: String,
}

#[derive(Deserialize)]
struct GhRateLimit {
    rate: GhRate,
}

#[derive(Deserialize)]
struct GhRate {
    limit: u32,
    remaining: u32,
    reset: i64,
}

impl From<GhRepo> for RemoteRepo {
    fn from(r: GhRepo) -> Self {
        RemoteRepo {
            full_name: r.full_name,
            owner: r.owner.login,
            name: r.name,
            clone_url: r.clone_url,
            ssh_url: r.ssh_url,
            default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
            is_fork: r.fork,
            upstream_full_name: r.parent.as_ref().map(|p| p.full_name.clone()),
            upstream_clone_url: r.parent.as_ref().map(|p| p.clone_url.clone()),
            description: r.description,
            is_private: r.private,
            is_archived: r.archived,
            updated_at: r
                .updated_at
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|dt| dt.with_timezone(&Utc)),
        }
    }
}

#[async_trait]
impl HostProvider for GitHubProvider {
    async fn validate_credentials(&self) -> Result<bool, GitrError> {
        let resp = self
            .client
            .get(self.url("/user"))
            .send()
            .await
            .map_err(|e| GitrError::ApiError {
                status: 0,
                message: e.to_string(),
            })?;
        Ok(resp.status().is_success())
    }

    async fn list_repos(&self) -> Result<Vec<RemoteRepo>, GitrError> {
        let gh_repos: Vec<GhRepo> = self.paginated_get("/user/repos", 100).await?;
        Ok(gh_repos.into_iter().map(RemoteRepo::from).collect())
    }

    async fn get_repo(&self, owner: &str, name: &str) -> Result<Option<RemoteRepo>, GitrError> {
        let url = self.url(&format!("/repos/{owner}/{name}"));
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitrError::ApiError {
                status: 0,
                message: e.to_string(),
            })?;

        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(GitrError::ApiError {
                status,
                message: body,
            });
        }

        let gh_repo: GhRepo = resp.json().await.map_err(|e| GitrError::ApiError {
            status: 0,
            message: format!("JSON parse error: {e}"),
        })?;
        Ok(Some(RemoteRepo::from(gh_repo)))
    }

    async fn list_branches(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<RemoteBranch>, GitrError> {
        let path = format!("/repos/{owner}/{name}/branches");
        let gh_branches: Vec<GhBranch> = self.paginated_get(&path, 100).await?;
        Ok(gh_branches
            .into_iter()
            .map(|b| RemoteBranch {
                name: b.name,
                sha: b.commit.sha,
                is_default: false,
            })
            .collect())
    }

    async fn fork_sync_status(
        &self,
        owner: &str,
        name: &str,
    ) -> Result<Vec<ForkSyncStatus>, GitrError> {
        // GitHub doesn't have a direct fork sync status API,
        // so we compare default branch commits via the compare endpoint.
        let repo = self.get_repo(owner, name).await?;
        let repo = match repo {
            Some(r) if r.is_fork => r,
            Some(_) => return Ok(Vec::new()),
            None => {
                return Err(GitrError::RepoNotFound {
                    name: format!("{owner}/{name}"),
                })
            }
        };

        let upstream = match &repo.upstream_full_name {
            Some(u) => u.clone(),
            None => return Ok(Vec::new()),
        };

        let branch = &repo.default_branch;
        let url = self.url(&format!(
            "/repos/{owner}/{name}/compare/{upstream}:{branch}...{branch}"
        ));
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitrError::ApiError {
                status: 0,
                message: e.to_string(),
            })?;

        if !resp.status().is_success() {
            return Ok(vec![ForkSyncStatus {
                branch: branch.clone(),
                behind_by: 0,
                ahead_by: 0,
            }]);
        }

        #[derive(Deserialize)]
        struct CompareResp {
            behind_by: u32,
            ahead_by: u32,
        }

        let compare: CompareResp =
            resp.json().await.map_err(|e| GitrError::ApiError {
                status: 0,
                message: format!("JSON parse error: {e}"),
            })?;

        Ok(vec![ForkSyncStatus {
            branch: branch.clone(),
            behind_by: compare.behind_by,
            ahead_by: compare.ahead_by,
        }])
    }

    async fn rate_limit_status(&self) -> Result<RateLimitInfo, GitrError> {
        let url = self.url("/rate_limit");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| GitrError::ApiError {
                status: 0,
                message: e.to_string(),
            })?;

        let rl: GhRateLimit = resp.json().await.map_err(|e| GitrError::ApiError {
            status: 0,
            message: format!("JSON parse error: {e}"),
        })?;

        let reset_at = DateTime::from_timestamp(rl.rate.reset, 0)
            .unwrap_or_else(Utc::now);

        Ok(RateLimitInfo {
            limit: rl.rate.limit,
            remaining: rl.rate.remaining,
            reset_at,
        })
    }

    fn kind(&self) -> HostKind {
        HostKind::GitHub
    }
}
