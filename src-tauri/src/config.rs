use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub domain: String,
    pub enabled: bool,
    pub header_name: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub proxy_port: u16,
    pub rules: Vec<Rule>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proxy_port: 7777,
            rules: Vec::new(),
        }
    }
}

pub fn app_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().context("no data dir")?;
    let dir = base.join("KeyProxy");
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_path() -> Result<PathBuf> {
    Ok(app_dir()?.join("config.json"))
}

pub fn load() -> Config {
    let path = match config_path() {
        Ok(p) => p,
        Err(_) => return Config::default(),
    };
    match fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path()?;
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(cfg)?;
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub struct ConfigStore {
    inner: RwLock<Config>,
}

impl ConfigStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(load()),
        }
    }

    pub fn snapshot(&self) -> Config {
        self.inner.read().unwrap().clone()
    }

    pub fn rules(&self) -> Vec<Rule> {
        self.inner.read().unwrap().rules.clone()
    }

    pub fn enabled_rule_for(&self, host: &str) -> Option<Rule> {
        self.inner
            .read()
            .unwrap()
            .rules
            .iter()
            .find(|r| r.enabled && r.domain.eq_ignore_ascii_case(host))
            .cloned()
    }

    pub fn port(&self) -> u16 {
        self.inner.read().unwrap().proxy_port
    }

    pub fn set_port(&self, port: u16) -> Result<()> {
        {
            let mut g = self.inner.write().unwrap();
            g.proxy_port = port;
            save(&g)?;
        }
        Ok(())
    }

    pub fn add_rule(&self, domain: String, label: String, header_name: String) -> Result<Rule> {
        let rule = Rule {
            id: Uuid::new_v4().to_string(),
            domain,
            enabled: true,
            header_name,
            label,
        };
        {
            let mut g = self.inner.write().unwrap();
            g.rules.push(rule.clone());
            save(&g)?;
        }
        Ok(rule)
    }

    pub fn update_rule(
        &self,
        id: &str,
        domain: String,
        label: String,
        header_name: String,
    ) -> Result<()> {
        let mut g = self.inner.write().unwrap();
        if let Some(r) = g.rules.iter_mut().find(|r| r.id == id) {
            r.domain = domain;
            r.label = label;
            r.header_name = header_name;
        }
        save(&g)
    }

    pub fn delete_rule(&self, id: &str) -> Result<Option<Rule>> {
        let mut g = self.inner.write().unwrap();
        let idx = g.rules.iter().position(|r| r.id == id);
        let removed = idx.map(|i| g.rules.remove(i));
        save(&g)?;
        Ok(removed)
    }

    pub fn toggle_rule(&self, id: &str, enabled: bool) -> Result<()> {
        let mut g = self.inner.write().unwrap();
        if let Some(r) = g.rules.iter_mut().find(|r| r.id == id) {
            r.enabled = enabled;
        }
        save(&g)
    }
}
