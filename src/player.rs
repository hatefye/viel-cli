use crate::api::Stream;
use anyhow::{bail, Context, Result};
use std::process::Command;

pub fn play(player: &str, stream: &Stream) -> Result<()> {
    let mut cmd = Command::new(player);
    cmd.arg(&stream.url);
    cmd.arg(format!("--force-media-title={}", stream.title));
    cmd.arg("--force-window=yes");
    if let Some(referer) = &stream.referer {
        cmd.arg(format!("--http-header-fields=Referer: {}", referer));
    }
    let status = cmd
        .status()
        .with_context(|| format!("could not launch {player}; install it and ensure it is on PATH"))?;
    if !status.success() {
        bail!("{player} exited with an error");
    }
    Ok(())
}
