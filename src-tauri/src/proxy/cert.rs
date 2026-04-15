use anyhow::{Context, Result};
use dashmap::DashMap;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa,
    KeyPair, KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::config::app_dir;
use crate::keychain;

pub struct CaMaterial {
    pub key_pem: String,
    pub cert_pem: String,
    pub cert_der: Vec<u8>,
    key_pair: KeyPair,
    cert: Certificate,
}

impl CaMaterial {
    pub fn key_pair(&self) -> &KeyPair {
        &self.key_pair
    }
    pub fn cert(&self) -> &Certificate {
        &self.cert
    }
}

pub struct CertStore {
    ca: RwLock<Arc<CaMaterial>>,
    server_configs: DashMap<String, Arc<ServerConfig>>,
}

impl CertStore {
    pub fn new() -> Result<Self> {
        let ca = load_or_create_ca()?;
        Ok(Self {
            ca: RwLock::new(Arc::new(ca)),
            server_configs: DashMap::new(),
        })
    }

    pub fn ca_cert_pem(&self) -> String {
        self.ca.read().unwrap().cert_pem.clone()
    }

    pub fn ca_cert_path(&self) -> Result<PathBuf> {
        let path = app_dir()?.join("ca.pem");
        fs::write(&path, self.ca_cert_pem())?;
        Ok(path)
    }

    pub fn regenerate(&self) -> Result<()> {
        keychain::delete_ca().ok();
        self.server_configs.clear();
        let ca = load_or_create_ca()?;
        *self.ca.write().unwrap() = Arc::new(ca);
        Ok(())
    }

    pub fn server_config_for(&self, domain: &str) -> Result<Arc<ServerConfig>> {
        if let Some(v) = self.server_configs.get(domain) {
            return Ok(v.clone());
        }
        let ca = self.ca.read().unwrap().clone();
        let cfg = Arc::new(build_server_config(&ca, domain)?);
        self.server_configs.insert(domain.to_string(), cfg.clone());
        Ok(cfg)
    }
}

fn load_or_create_ca() -> Result<CaMaterial> {
    if let Some((key_pem, cert_pem)) = keychain::load_ca() {
        if let Ok(material) = materialize_ca(&key_pem, &cert_pem) {
            return Ok(material);
        }
    }
    let (key_pem, cert_pem) = generate_ca()?;
    keychain::save_ca(&key_pem, &cert_pem)?;
    materialize_ca(&key_pem, &cert_pem)
}

fn generate_ca() -> Result<(String, String)> {
    let key_pair = KeyPair::generate().context("generate CA keypair")?;
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, "KeyProxy Local CA");
    dn.push(DnType::OrganizationName, "KeyProxy");
    params.distinguished_name = dn;
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    let cert = params.self_signed(&key_pair)?;
    Ok((key_pair.serialize_pem(), cert.pem()))
}

fn materialize_ca(key_pem: &str, cert_pem: &str) -> Result<CaMaterial> {
    let key_pair = KeyPair::from_pem(key_pem).context("parse CA key pem")?;
    let params = CertificateParams::from_ca_cert_pem(cert_pem).context("parse CA cert pem")?;
    let cert = params.self_signed(&key_pair)?;
    let cert_der = cert.der().to_vec();
    Ok(CaMaterial {
        key_pem: key_pem.to_string(),
        cert_pem: cert_pem.to_string(),
        cert_der,
        key_pair,
        cert,
    })
}

fn build_server_config(ca: &CaMaterial, domain: &str) -> Result<ServerConfig> {
    let leaf_key = KeyPair::generate().context("generate leaf key")?;
    let mut params = CertificateParams::new(vec![domain.to_string()])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, domain);
    params.distinguished_name = dn;
    params.use_authority_key_identifier_extension = true;

    let leaf = params
        .signed_by(&leaf_key, ca.cert(), ca.key_pair())
        .context("sign leaf cert")?;

    let leaf_der = CertificateDer::from(leaf.der().to_vec());
    let ca_der = CertificateDer::from(ca.cert_der.clone());
    let key_der = PrivatePkcs8KeyDer::from(leaf_key.serialize_der());
    let key_any = PrivateKeyDer::Pkcs8(key_der);

    let signing_key = rustls::crypto::ring::sign::any_supported_type(&key_any)
        .context("load signing key into rustls")?;

    let certified = CertifiedKey::new(vec![leaf_der, ca_der], signing_key);

    let resolver = SingleCertResolver(Arc::new(certified));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));
    Ok(config)
}

#[derive(Debug)]
struct SingleCertResolver(Arc<CertifiedKey>);

impl rustls::server::ResolvesServerCert for SingleCertResolver {
    fn resolve(
        &self,
        _client_hello: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<CertifiedKey>> {
        Some(self.0.clone())
    }
}

pub fn trust_ca_via_security_cli(cert_path: &PathBuf) -> Result<()> {
    let script = format!(
        r#"do shell script "security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain '{path}'" with administrator privileges"#,
        path = cert_path.display()
    );
    let out = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()?;
    if !out.status.success() {
        let s = String::from_utf8_lossy(&out.stderr);
        anyhow::bail!("trust CA failed: {s}");
    }
    Ok(())
}

pub fn is_ca_trusted() -> bool {
    let out = std::process::Command::new("security")
        .args([
            "find-certificate",
            "-c",
            "KeyProxy Local CA",
            "/Library/Keychains/System.keychain",
        ])
        .output();
    matches!(out, Ok(o) if o.status.success())
}

pub fn untrust_ca() -> Result<()> {
    let script = r#"do shell script "security delete-certificate -c 'KeyProxy Local CA' /Library/Keychains/System.keychain" with administrator privileges"#;
    let _ = std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    Ok(())
}
