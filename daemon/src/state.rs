use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::proxy::cert::CertStore;

#[derive(Clone, Debug)]
pub struct ActiveRule {
    pub header_name: String,
    pub credential: String,
}

pub struct State {
    rules: RwLock<HashMap<String, ActiveRule>>, // domain -> rule
    ca: RwLock<Option<Arc<CertStore>>>,
}

impl State {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(HashMap::new()),
            ca: RwLock::new(None),
        }
    }

    pub fn set_rules(&self, rules: Vec<(String, ActiveRule)>) {
        let mut g = self.rules.write().unwrap();
        g.clear();
        for (d, r) in rules {
            g.insert(d.to_ascii_lowercase(), r);
        }
    }

    pub fn rule_for(&self, host: &str) -> Option<ActiveRule> {
        self.rules.read().unwrap().get(&host.to_ascii_lowercase()).cloned()
    }

    pub fn set_ca(&self, store: Arc<CertStore>) {
        *self.ca.write().unwrap() = Some(store);
    }

    pub fn ca(&self) -> Option<Arc<CertStore>> {
        self.ca.read().unwrap().clone()
    }
}
