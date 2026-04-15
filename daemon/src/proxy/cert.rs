use anyhow::{Context, Result};
use dashmap::DashMap;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DistinguishedName, DnType, IsCa, KeyPair,
    KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::sign::CertifiedKey;
use rustls::ServerConfig;
use std::sync::Arc;

pub struct CertStore {
    cert_der: Vec<u8>,
    key_pair: KeyPair,
    cert: Certificate,
    server_configs: DashMap<String, Arc<ServerConfig>>,
}

impl CertStore {
    pub fn from_pem(key_pem: &str, cert_pem: &str) -> Result<Self> {
        let key_pair = KeyPair::from_pem(key_pem).context("parse CA key pem")?;
        let params = CertificateParams::from_ca_cert_pem(cert_pem).context("parse CA cert pem")?;
        let cert = params.self_signed(&key_pair)?;
        let cert_der = cert.der().to_vec();
        Ok(Self {
            cert_der,
            key_pair,
            cert,
            server_configs: DashMap::new(),
        })
    }

    pub fn server_config_for(&self, domain: &str) -> Result<Arc<ServerConfig>> {
        if let Some(v) = self.server_configs.get(domain) {
            return Ok(v.clone());
        }
        let cfg = Arc::new(build_server_config(self, domain)?);
        self.server_configs.insert(domain.to_string(), cfg.clone());
        Ok(cfg)
    }
}

fn build_server_config(ca: &CertStore, domain: &str) -> Result<ServerConfig> {
    let leaf_key = KeyPair::generate().context("generate leaf key")?;
    let mut params = CertificateParams::new(vec![domain.to_string()])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, domain);
    params.distinguished_name = dn;
    params.use_authority_key_identifier_extension = true;

    let leaf = params
        .signed_by(&leaf_key, &ca.cert, &ca.key_pair)
        .context("sign leaf cert")?;

    let leaf_der = CertificateDer::from(leaf.der().to_vec());
    let ca_der = CertificateDer::from(ca.cert_der.clone());
    let key_der = PrivatePkcs8KeyDer::from(leaf_key.serialize_der());
    let signing_key = rustls::crypto::ring::sign::any_supported_type(&PrivateKeyDer::Pkcs8(key_der))
        .context("load signing key into rustls")?;

    let certified = CertifiedKey::new(vec![leaf_der, ca_der], signing_key);
    let resolver = SingleCertResolver(Arc::new(certified));
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(resolver));
    Ok(config)
}

pub fn generate_ca() -> Result<(String, String)> {
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
