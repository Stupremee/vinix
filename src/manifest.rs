use color_eyre::{eyre::bail, Result};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ManifestEntry {
    pub owner: String,
    pub repo: String,
    pub name: Option<String>,
}

pub async fn read_manifest(path: &Path) -> Result<Vec<ManifestEntry>> {
    let contents = tokio::fs::read_to_string(path).await?;

    let mut entries = vec![];
    for (idx, line) in contents.lines().enumerate() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((owner, rest)) = line.split_once('/') else {
            // start line numbers at 1
            bail!("line {} was invalid", idx + 1);
        };

        let (repo, name) = match rest.split_once(':') {
            Some((repo, name)) => (repo, Some(name)),
            None => (rest, None),
        };

        entries.push(ManifestEntry {
            owner: owner.trim().to_string(),
            repo: repo.trim().to_string(),
            name: name.map(|s| s.trim().to_string()),
        })
    }

    Ok(entries)
}
