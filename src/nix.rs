use color_eyre::{eyre::bail, Result};
use tokio::process::Command;

pub async fn prefetch_url(url: &str) -> Result<String> {
    let cmd = Command::new("nix-prefetch-url")
        .args(["--type", "sha256", url])
        .output()
        .await?;

    if !cmd.status.success() {
        bail!(
            "nix-prefetch-url failed: {}",
            String::from_utf8_lossy(&cmd.stderr)
        );
    }

    let hash = String::from_utf8(cmd.stdout)?;
    Ok(hash)
}
