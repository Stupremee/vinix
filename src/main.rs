mod github;
mod manifest;
mod nix;

use alejandra::format::Status;
use clap::Parser;
use color_eyre::Result;
use github::GithubClient;
use manifest::ManifestEntry;
use std::{path::PathBuf, sync::Arc};
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the manifest file
    #[arg()]
    manifest: PathBuf,
    /// Github API token.
    ///
    /// Used to fetch the latest commit hash, and avoid rate limits.
    #[arg(long, env = "GITHUB_TOKEN")]
    github_api_token: String,
    /// The file to write the generated expression to.
    #[arg(long, short)]
    file: PathBuf,
    /// Do not format the generated expression.
    #[arg(long, default_value = "false")]
    no_format: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let manifest = manifest::read_manifest(&args.manifest).await?;

    let client = GithubClient::new(args.github_api_token)?;

    // use a semaphore to only run 50 tasks at a time,
    // otherwise we would get "Too many files are open" error
    let semaphore = Arc::new(Semaphore::new(50));

    let mut tasks = JoinSet::new();
    for entry in manifest.into_iter() {
        tasks.spawn(generate_pkg(client.clone(), entry, semaphore.clone()));
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
            Err(err) => println!("{err}"),
        }
    }

    expr.push('}');

    let expr = if !args.no_format {
        let (status, code) = alejandra::format::in_memory(args.file.display().to_string(), expr);
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

    tokio::fs::write(&args.file, expr).await?;

    Ok(())
}

async fn generate_pkg(
    client: GithubClient,
    entry: ManifestEntry,
    semaphore: Arc<Semaphore>,
) -> Result<String> {
    let _permit = semaphore.acquire().await?;

    let commit = client.get_latest_commit(&entry).await?;
    let hash = nix::prefetch_url(&commit.tarball_url).await?;

    let name = entry
        .name
        .clone()
        .unwrap_or_else(|| entry.repo.replace('.', "-"))
        .to_lowercase();

    let date = commit.date.format("%Y-%m-%d");

    let expr = format!(
        r#"  {name} = buildVimPluginFrom2Nix {{
    pname = "{name}"; # Manifest entry: "{owner}/{repo}"
    version = "{date}";
    src = fetchurl {{
      url = "{tarball_url}";
      sha256 = "{trimmed_hash}";
    }};
  }};
"#,
        tarball_url = commit.tarball_url,
        owner = entry.owner,
        repo = entry.repo,
        trimmed_hash = hash.trim(),
    );

    Ok(expr)
}
