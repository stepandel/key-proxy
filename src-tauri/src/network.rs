use anyhow::{Context, Result};
use std::process::Command;

fn run(cmd: &mut Command) -> Result<String> {
    let out = cmd.output().context("failed to spawn networksetup")?;
    if !out.status.success() {
        let s = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow::anyhow!("networksetup error: {s}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn list_services() -> Result<Vec<String>> {
    let out = run(Command::new("networksetup").arg("-listallnetworkservices"))?;
    Ok(out
        .lines()
        .skip(1)
        .filter(|l| !l.starts_with('*') && !l.trim().is_empty())
        .map(|l| l.trim().to_string())
        .collect())
}

pub fn enable_proxy(host: &str, port: u16) -> Result<()> {
    for svc in list_services()? {
        // Ignore errors for individual services (some may be disabled/virtual)
        let _ = run(Command::new("networksetup")
            .arg("-setwebproxy")
            .arg(&svc)
            .arg(host)
            .arg(port.to_string()));
        let _ = run(Command::new("networksetup")
            .arg("-setsecurewebproxy")
            .arg(&svc)
            .arg(host)
            .arg(port.to_string()));
        let _ = run(Command::new("networksetup")
            .arg("-setwebproxystate")
            .arg(&svc)
            .arg("on"));
        let _ = run(Command::new("networksetup")
            .arg("-setsecurewebproxystate")
            .arg(&svc)
            .arg("on"));
    }
    Ok(())
}

pub fn disable_proxy() -> Result<()> {
    for svc in list_services()? {
        let _ = run(Command::new("networksetup")
            .arg("-setwebproxystate")
            .arg(&svc)
            .arg("off"));
        let _ = run(Command::new("networksetup")
            .arg("-setsecurewebproxystate")
            .arg(&svc)
            .arg("off"));
    }
    Ok(())
}
