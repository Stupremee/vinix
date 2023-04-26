use chrono::FixedOffset;
use color_eyre::{
    eyre::{bail, Context},
    Result,
};
use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use reqwest::{header::HeaderMap, Client};

use crate::config::{Plugin, Repository};

type GitObjectID = String;
type DateTime = chrono::DateTime<FixedOffset>;

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub tarball_url: String,
    pub version: String,
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/github/schema.graphql",
    query_path = "src/github/get_latest_tarball.graphql",
    response_derives = "Debug",
    variables_derives = "Debug"
)]
struct GetTarball;

#[derive(Clone, Debug)]
pub struct GithubClient {
    client: Client,
}

impl GithubClient {
    pub fn new(github_api_token: Option<String>) -> Result<Self> {
        let headers = if let Some(github_api_token) = github_api_token {
            std::iter::once((
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", github_api_token))
                    .unwrap(),
            ))
            .collect()
        } else {
            HeaderMap::new()
        };

        let client = Client::builder()
            .user_agent("graphql-rust/0.10.0")
            .default_headers(headers)
            .build()?;

        Ok(Self { client })
    }

    pub async fn get_latest_commit(&self, plugin: &Plugin) -> Result<CommitInfo> {
        let (commit, version) = match plugin.rev {
            Some(ref rev) => (rev.to_string(), rev.to_string()),
            None => {
                let (rev, date) = self
                    .fetch_latest_commit(&plugin.repo)
                    .await
                    .context("Fetching commit from GitHub API")?;
                let ver = date.format("%Y-%m-%d").to_string();

                (rev, ver)
            }
        };

        let tarball_url = format!(
            "https://github.com/{}/{}/archive/{}.tar.gz",
            plugin.repo.owner, plugin.repo.name, commit
        );

        Ok(CommitInfo {
            tarball_url,
            version,
        })
    }

    async fn fetch_latest_commit(&self, repo: &Repository) -> Result<(String, DateTime)> {
        let variables = get_tarball::Variables {
            repo: repo.name.to_string(),
            owner: repo.owner.to_string(),
        };

        let response = post_graphql::<GetTarball, _>(
            &self.client,
            "https://api.github.com/graphql",
            variables,
        )
        .await?;

        if let Some(errors) = response.errors.filter(|e| !e.is_empty()) {
            let error = errors.first().map(ToString::to_string).unwrap();

            bail!("entry {}/{} failed: {}", repo.owner, repo.name, error)
        }

        let data: get_tarball::ResponseData = response.data.unwrap();

        let target = data
            .repository
            .unwrap()
            .default_branch_ref
            .unwrap()
            .target
            .unwrap();

        let commit = match target {
            get_tarball::GetTarballRepositoryDefaultBranchRefTarget::Commit(commit) => {
                (commit.oid, commit.committed_date)
            }
            _ => bail!("target is not commit"),
        };

        Ok(commit)
    }
}
