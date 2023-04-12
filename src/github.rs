use chrono::FixedOffset;
use color_eyre::{eyre::bail, Result};
use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use reqwest::Client;

use crate::manifest::ManifestEntry;

#[allow(clippy::upper_case_acronyms)]
type URI = String;

type DateTime = chrono::DateTime<FixedOffset>;

#[derive(Clone, Debug)]
pub struct CommitInfo {
    pub tarball_url: String,
    pub date: chrono::DateTime<FixedOffset>,
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
    pub fn new(github_api_token: String) -> Result<Self> {
        let client = Client::builder()
            .user_agent("graphql-rust/0.10.0")
            .default_headers(
                std::iter::once((
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", github_api_token))
                        .unwrap(),
                ))
                .collect(),
            )
            .build()?;

        Ok(Self { client })
    }

    pub async fn get_latest_commit(&self, entry: &ManifestEntry) -> Result<CommitInfo> {
        let variables = get_tarball::Variables {
            repo: entry.repo.to_string(),
            owner: entry.owner.to_string(),
        };

        let response = post_graphql::<GetTarball, _>(
            &self.client,
            "https://api.github.com/graphql",
            variables,
        )
        .await?;

        if let Some(errors) = response.errors.filter(|e| !e.is_empty()) {
            let error = errors.first().map(ToString::to_string).unwrap();

            bail!("entry {}/{} failed: {}", entry.owner, entry.repo, error)
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
            get_tarball::GetTarballRepositoryDefaultBranchRefTarget::Commit(commit) => CommitInfo {
                tarball_url: commit.tarball_url,
                date: commit.committed_date,
            },
            _ => bail!("target is not commit"),
        };

        Ok(commit)
    }
}
