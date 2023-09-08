use crate::*;
use alloy_primitives::{hex::FromHex, Address};
use hapi_iron_oxide::{seal, unseal};
use lazy_static::lazy_static;
use rand::RngCore;

lazy_static! {
    static ref COOKIE_VERSION: &'static str = "~2";
}

#[derive(Deserialize, Serialize)]
struct SessionCookie {
    address: String,
}

#[derive(Debug)]
pub struct Session {
    pub address: Address,
}

impl Session {
    pub fn from_cookie(iron_cookie: &str, secret: &str) -> Result<Self> {
        if let Some((cookie, _version)) = iron_cookie.split_once(*COOKIE_VERSION) {
            let unsealed = unseal(cookie.to_string(), secret, Default::default())?;
            let session_cookie = serde_json::from_str::<SessionCookie>(&unsealed)?;
            Ok(Self {
                address: Address::parse_checksummed(session_cookie.address, None)?,
            })
        } else {
            bail!("Invalid iron cookie format")
        }
    }

    pub fn to_cookie(&self, secret: &str) -> String {
        let cookie_data = serde_json::to_string(&SessionCookie {
            address: self.address.to_string(),
        })
        .unwrap();

        let mut cookie = seal::<32, 32, _>(cookie_data, secret, Default::default()).unwrap();
        cookie.push_str(*COOKIE_VERSION);
        cookie
    }
}

impl Default for Session {
    fn default() -> Self {
        let mut bytes = [0; 20];
        rand::thread_rng().fill_bytes(&mut bytes);
        let address =
            Address::from_hex(hex::encode(bytes)).expect("20 bytes in hex should be valid address");
        Self { address }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_session() -> Result<()> {
        let session_secret = "asdjhashjdasjhdashjkdhjkasdjkhsdfghsdf";
        let default_session = Session::default();
        let cookie = default_session.to_cookie(session_secret);

        let session = Session::from_cookie(&cookie, session_secret)?;
        assert_eq!(session.address, default_session.address);
        Ok(())
    }
}
