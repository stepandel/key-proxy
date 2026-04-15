use anyhow::{anyhow, Result};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};

const SERVICE: &str = "keyproxy";
const CA_SERVICE: &str = "keyproxy-ca";
const CA_KEY_ACCOUNT: &str = "ca-private-key";
const CA_CERT_ACCOUNT: &str = "ca-certificate";

pub fn set_credential(domain: &str, value: &str) -> Result<()> {
    set_generic_password(SERVICE, domain, value.as_bytes())
        .map_err(|e| anyhow!("keychain set: {e}"))
}

pub fn get_credential(domain: &str) -> Option<String> {
    match get_generic_password(SERVICE, domain) {
        Ok(bytes) => String::from_utf8(bytes).ok(),
        Err(_) => None,
    }
}

pub fn delete_credential(domain: &str) -> Result<()> {
    match delete_generic_password(SERVICE, domain) {
        Ok(_) => Ok(()),
        Err(e) => {
            let code = e.code();
            // errSecItemNotFound = -25300
            if code == -25300 {
                Ok(())
            } else {
                Err(anyhow!("keychain delete: {e}"))
            }
        }
    }
}

pub fn save_ca(key_pem: &str, cert_pem: &str) -> Result<()> {
    set_generic_password(CA_SERVICE, CA_KEY_ACCOUNT, key_pem.as_bytes())
        .map_err(|e| anyhow!("keychain save CA key: {e}"))?;
    set_generic_password(CA_SERVICE, CA_CERT_ACCOUNT, cert_pem.as_bytes())
        .map_err(|e| anyhow!("keychain save CA cert: {e}"))?;
    Ok(())
}

pub fn load_ca() -> Option<(String, String)> {
    let k = get_generic_password(CA_SERVICE, CA_KEY_ACCOUNT).ok()?;
    let c = get_generic_password(CA_SERVICE, CA_CERT_ACCOUNT).ok()?;
    Some((String::from_utf8(k).ok()?, String::from_utf8(c).ok()?))
}

pub fn delete_ca() -> Result<()> {
    let _ = delete_generic_password(CA_SERVICE, CA_KEY_ACCOUNT);
    let _ = delete_generic_password(CA_SERVICE, CA_CERT_ACCOUNT);
    Ok(())
}
