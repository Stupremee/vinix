mod config;
mod github;
mod nix;

use alejandra::format::Status;
use clap::Parser;
use color_eyre::{eyre::Context, Result};
use github::GithubClient;
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the config file
    #[arg(long, short, default_value = "./vimPlugins.toml")]
    config: PathBuf,
    /// Github API token.
    ///
    /// Used to fetch the latest commit hash, and avoid rate limits.
    #[arg(long, env = "GITHUB_TOKEN")]
    github_api_token: Option<String>,
    /// The file to write the generated expression to.
    ///
    /// This will override the setting in the config file.
    #[arg(long)]
    file: Option<PathBuf>,
    /// Do not format the generated expression.
    #[arg(long, default_value = "false")]
    no_format: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let config = config::read_config(&args.config)
        .await
        .with_context(|| format!("Reading config from: {}", args.config.display()))?;

    let client = GithubClient::new(args.github_api_token)?;

    // use a semaphore to only run 50 tasks at a time,
    // otherwise we would get "Too many files are open" error
    let semaphore = Arc::new(Semaphore::new(50));

    let mut tasks = JoinSet::new();
    for (name, plugin) in config.plugins.into_iter() {
        tasks.spawn(generate_pkg(
            name,
            plugin,
            client.clone(),
            semaphore.clone(),
        ));
    }

    let mut expr = "\
    { buildVimPluginFrom2Nix, fetchurl }:
    {\n"
    .to_string();

    while let Some(res) = tasks.join_next().await {
        let res = res?;

        match res {
            Ok(res) => {
                expr.push_str(&res);
            }
            Err(err) => eprintln!("{err:?}"),
        }
    }

    expr.push('}');

    let file = args.file.unwrap_or(config.file);
    let expr = if !args.no_format {
        let (status, code) = alejandra::format::in_memory(file.display().to_string(), expr);
        if let Status::Error(err) = status {
            panic!(
                "product invalid Nix code, this is an internal error: {}",
                err
            );
        }

        code
    } else {
        expr
    };

    tokio::fs::write(&file, expr)
        .await
        .with_context(|| format!("Saving generated Nix code to {}", file.display()))?;

    Ok(())
}

async fn generate_pkg(
    name: String,
    plugin: config::Plugin,
    client: GithubClient,
    semaphore: Arc<Semaphore>,
) -> Result<String> {
    let _permit = semaphore.acquire().await.context("Acquiring semaphore")?;

    let commit = client.get_latest_commit(&plugin).await.with_context(|| {
        format!(
            "Getting latest commit for repo: {}/{}",
            plugin.repo.owner, plugin.repo.name
        )
    })?;

    let hash = nix::prefetch_url(&commit.tarball_url)
        .await
        .with_context(|| format!("Failed prefetching url: {}", commit.tarball_url))?;

    let name = name.replace('.', "-");
    let version = commit.version;

    let expr = format!(
        r#"  {name} = buildVimPluginFrom2Nix {{
    pname = "{name}"; # Manifest entry: "{owner}/{repo}"
    version = "{version}";
    src = fetchurl {{
      url = "{tarball_url}";
      sha256 = "{trimmed_hash}";
    }};
  }};
"#,
        tarball_url = commit.tarball_url,
        owner = plugin.repo.owner,
        repo = plugin.repo.name,
        trimmed_hash = hash.trim(),
    );

    Ok(expr)
}
