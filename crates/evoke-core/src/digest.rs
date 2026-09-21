//! SHA-256 over sorted `(path, SHA-256(bytes))` pairs — the design's `h1`. In: pairs, or the files already hashed
//! by a host that holds the bytes. Out: `Digest`, the same for trust, the store and the plan.

use std::fmt::{self, Write as _};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use crate::name::RelPath;

/// A SHA-256, displayed and serialized as `h1:<hex>`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Digest([u8; 32]);

/// `h1` of a reflex directory: `sha256sum`'s own line per file, paths in byte order, then the SHA-256 of those lines.
#[must_use]
pub fn digest(pairs: &[(RelPath, &[u8])]) -> Digest {
    let hashed: Vec<(RelPath, Digest)> = pairs
        .iter()
        .map(|(path, bytes)| (path.clone(), Digest::of(bytes)))
        .collect();
    compose(&hashed)
}

/// The same `h1` from each file's own SHA-256, for a host that hashes the bytes itself; the recipe lives here alone.
#[must_use]
pub fn compose(hashed: &[(RelPath, Digest)]) -> Digest {
    let mut lines: Vec<(&str, Digest)> = hashed
        .iter()
        .map(|(path, hash)| (path.as_str(), *hash))
        .collect();
    lines.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let text = lines.iter().fold(String::new(), |mut text, (path, hash)| {
        let _ = writeln!(text, "{}  {path}", hash.hex());
        text
    });
    Digest::of(text.as_bytes())
}

impl Digest {
    /// The SHA-256 of bytes: a file's line in `h1`, the plan's over its JSON.
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    fn hex(self) -> String {
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }
}

impl TryFrom<String> for Digest {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        let invalid = || format!("\"{text}\" is not a digest: h1:<64 hex>");
        let hex = text
            .strip_prefix("h1:")
            .filter(|hex| hex.len() == 64)
            .ok_or_else(invalid)?;
        let mut bytes = [0u8; 32];
        for (byte, i) in bytes.iter_mut().zip((0..64).step_by(2)) {
            let pair = hex
                .get(i..i + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok());
            *byte = pair.ok_or_else(invalid)?;
        }
        Ok(Self(bytes))
    }
}

impl From<Digest> for String {
    fn from(digest: Digest) -> Self {
        digest.to_string()
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "h1:{}", self.hex())
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn of_bytes_is_sha256() {
        assert_eq!(
            Digest::of(b"hello").to_string(),
            "h1:2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn h1_reproduces_the_shell_recipe() {
        // find . -type f | sed 's|^\./||' | LC_ALL=C sort | xargs sha256sum | sha256sum, over b/c = "y" and a = "x".
        let pairs = [
            (RelPath::new("b/c").unwrap(), b"y".as_slice()),
            (RelPath::new("a").unwrap(), b"x".as_slice()),
        ];
        assert_eq!(
            digest(&pairs).to_string(),
            "h1:4fee141d00d6e8d53a68f8aefe86aa45ff1297afffa0709f5d3a3136392e731d"
        );
        assert_eq!(
            digest(&[]).to_string(),
            "h1:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let hashed = [
            (RelPath::new("b/c").unwrap(), Digest::of(b"y")),
            (RelPath::new("a").unwrap(), Digest::of(b"x")),
        ];
        assert_eq!(compose(&hashed), digest(&pairs));
    }

    #[test]
    fn round_trips_through_its_text() {
        let digest = Digest::of(b"x");
        let json = serde_json::to_string(&digest).unwrap();
        assert_eq!(serde_json::from_str::<Digest>(&json).unwrap(), digest);
        assert!(serde_json::from_str::<Digest>("\"h1:zz\"").is_err());
        assert!(serde_json::from_str::<Digest>("\"abc\"").is_err());
    }
}
