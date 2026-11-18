use anyhow::{Context, Result};
use std::process::Command;

pub fn execute_command(cmd_str: &str) -> Result<()> {
    let trimmed = cmd_str.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let mut child = Command::new("sh")
        .arg("-c")
        .arg(trimmed)
        .spawn()
        .with_context(|| format!("Failed to spawn shell command '{}'", trimmed))?;

    std::thread::spawn(move || {
        let _ = child.wait();
    });

    Ok(())
}
