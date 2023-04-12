mod github;
mod manifest;
mod nix;

use clap::Parser;
use color_eyre::Result;
use github::GithubClient;
use manifest::ManifestEntry;
use std::path::PathBuf;
use tokio::task::JoinSet;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the manifest file
    #[arg()]
    manifest: PathBuf,
    #[arg(long, env = "GITHUB_TOKEN")]
    github_api_token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let manifest = manifest::read_manifest(&args.manifest).await?;

    let client = GithubClient::new(args.github_api_token)?;

    let mut tasks = JoinSet::new();
    for entry in manifest.into_iter() {
        tasks.spawn(generate_pkg(client.clone(), entry));
    }

    while let Some(res) = tasks.join_next().await {
        let res = res?;

        match res {
            Ok(res) => println!("{}", res),
            Err(err) => println!("{}", err),
        }
    }

    Ok(())
}

async fn generate_pkg(client: GithubClient, entry: ManifestEntry) -> Result<String> {
    let commit = client.get_latest_commit(&entry).await?;
    let hash = nix::prefetch_url(commit.tarball_url).await?;

    let name = entry
        .name
        .clone()
        .unwrap_or_else(|| entry.repo.replace('.', "-"));

    //     let expr = format!(
    //         r#"\
    // {0} = buildVimPluginFrom2Nix {
    //     pname = "{0}";
    //     version = "";
    // }"#,
    //         name,
    //     );
    Ok(format!(
        "{}/{}: {} {}",
        entry.owner, entry.repo, hash, commit.date
    ))
}
