use color_eyre::Result;
use serde::{de, Deserialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub file: PathBuf,
    #[serde(flatten)]
    pub plugins: HashMap<String, Plugin>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Plugin {
    pub repo: Repository,
    pub rev: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Repository {
    pub owner: String,
    pub name: String,
}

impl<'de> Deserialize<'de> for Repository {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let mut split = s.splitn(2, '/');

        let Some(owner) = split.next() else {
            return Err(<D::Error as de::Error>::custom("invalid repository, format is owner/name"));
        };

        let Some(name) = split.next() else {
            return Err(<D::Error as de::Error>::custom("invalid repository, format is owner/name"));
        };

        Ok(Self {
            owner: owner.to_string(),
            name: name.to_string(),
        })
    }
}

pub async fn read_config(path: &Path) -> Result<Config> {
    let file = tokio::fs::read_to_string(path).await?;
    let config = toml::from_str(&file)?;

    Ok(config)
}
