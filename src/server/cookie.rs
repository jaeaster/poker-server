use alloy_primitives::{address, Address};
use eyre::{bail, Result};
use hapi_iron_oxide::unseal;
use serde::Deserialize;

#[derive(Deserialize)]
struct SessionCookie {
    address: String,
}

#[derive(Debug)]
pub struct Session {
    pub address: Address,
}

impl Session {
    pub fn from_cookie(iron_cookie: &str, secret: &str) -> Result<Self> {
        if let Some((cookie, _version)) = iron_cookie.split_once('~') {
            let unsealed = unseal(cookie.to_string(), secret, Default::default())?;
            let session_cookie = serde_json::from_str::<SessionCookie>(&unsealed)?;
            Ok(Self {
                address: Address::parse_checksummed(session_cookie.address, None)?,
            })
        } else {
            bail!("Invalid iron cookie format")
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self {
            address: address!("760f35dc48a52320d905b3ef1df7bb29abd4484e"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iron_session() -> Result<()> {
        let session_secret = "asdjhashjdasjhdashjkdhjkasdjkhsdfghsdf";
        let cookie = "Fe26.2*1*8dc93bbc3f6bebfa3ff420ae8c5c7759a82b37ba3e03cd93230650157f977aa2*iaulAH2srSxJQMYMmHudVQ*tCTDJGo3SSJDbY4T2rdDnb-X6hCDNaRnK-lpOkkviQ1_gnP4ordWDtLi8WTyCcVUGvdNwGSuBx1ReNs2xMb8Z466JyPlmmQvIDApwlTH1qzxkBmph7zK7cVSoR5xvRV_DIGfMsI8fl4ee7XIheMdHA*1695852330243*47e9ef9fb2ed30abc8e30fce62b4a2952ab15a1f40b79e98d80367316ba35ca1*161OhrWK-HSCqpqeYiK5Y40w4IySGPyc7DCHH62ixSk~2";

        let session = Session::from_cookie(cookie, session_secret)?;
        assert_eq!(session.address, Session::default().address);
        Ok(())
    }
}
