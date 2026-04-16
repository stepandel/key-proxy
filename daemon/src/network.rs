use std::process::Command;

pub fn list_services() -> Vec<String> {
    let out = match Command::new("networksetup")
        .arg("-listallnetworkservices")
        .output()
    {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        _ => return Vec::new(),
    };
    out.lines()
        .skip(1)
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('*'))
        .collect()
}

pub fn disable_all() {
    for svc in list_services() {
        let _ = Command::new("networksetup")
            .args(["-setwebproxystate", &svc, "off"])
            .status();
        let _ = Command::new("networksetup")
            .args(["-setsecurewebproxystate", &svc, "off"])
            .status();
    }
}
