use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use md5::{Digest, Md5};
use quick_xml::escape::unescape;
use quick_xml::events::Event;
use quick_xml::Reader;
use sha1::Sha1;
use sha2::Sha512;

use crate::ReconnectCredentialVerifier;

/// Minimal storage-independent account lookup contract.
pub trait AccountRepository {
    type Account;
    type Error;

    fn authenticate(&self, username: &str, password: &str) -> Result<Self::Account, Self::Error>;
}

/// Adapts any account repository to reconnect credential verification.
pub struct RepositoryCredentialVerifier<R> {
    repository: R,
}

impl<R> RepositoryCredentialVerifier<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
}

/// A legacy account identity backed by its stored username, not a numeric ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyAccountIdentity {
    username: String,
}

impl LegacyAccountIdentity {
    pub fn new(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

#[derive(Clone)]
enum LegacyCredential {
    Plain(String),
    Md5(String),
    Sha1(String),
    Sha512(String),
}

/// The identity and credential needed by the account verification boundary.
struct StoredAccount {
    username: String,
    credential: LegacyCredential,
}

/// Read-only adapter for ServUO's `accounts.xml` format.
pub struct LegacyXmlAccountRepository {
    accounts: Vec<StoredAccount>,
    lookup: HashMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyAccountError {
    Io,
    MalformedXml,
    InvalidAccountData,
    AmbiguousUsername,
    InvalidInput,
    UnknownUsername,
    PasswordMismatch,
}

impl LegacyXmlAccountRepository {
    /// Load accounts from the caller-selected file. The source is never opened for writing.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, LegacyAccountError> {
        let xml = fs::read_to_string(path).map_err(|_| LegacyAccountError::Io)?;
        Self::parse(&xml)
    }

    fn parse(xml: &str) -> Result<Self, LegacyAccountError> {
        let mut reader = Reader::from_str(xml);
        let mut accounts = Vec::new();
        let mut stack: Vec<String> = Vec::new();
        let mut current: Option<HashMap<String, String>> = None;
        let mut current_field: Option<String> = None;
        let mut root_seen = false;
        let mut root_closed = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(event)) => {
                    let name = event.name().as_ref().to_owned();
                    if stack.is_empty() {
                        if root_seen || name != "accounts" {
                            return Err(LegacyAccountError::MalformedXml);
                        }
                        root_seen = true;
                    } else if stack.as_slice() == ["accounts"] {
                        if name != "account" {
                            return Err(LegacyAccountError::MalformedXml);
                        }
                        current = Some(HashMap::new());
                    } else if current.is_some() {
                        if stack.len() == 2 && is_credential_field(&name) {
                            if current_field.is_some() {
                                return Err(LegacyAccountError::InvalidAccountData);
                            }
                            let fields = current
                                .as_mut()
                                .ok_or(LegacyAccountError::InvalidAccountData)?;
                            if fields.insert(name.clone(), String::new()).is_some() {
                                return Err(LegacyAccountError::InvalidAccountData);
                            }
                            current_field = Some(name.clone());
                        } else if current_field.is_some() || name == "account" || name == "accounts"
                        {
                            return Err(LegacyAccountError::InvalidAccountData);
                        }
                    } else {
                        return Err(LegacyAccountError::MalformedXml);
                    }
                    stack.push(name);
                }
                Ok(Event::Empty(event)) => {
                    let qualified_name = event.name();
                    let name = qualified_name.as_ref();
                    if stack.is_empty() && name == "accounts" && !root_seen {
                        root_seen = true;
                        root_closed = true;
                    } else if stack.as_slice() == ["accounts"] && name == "account" {
                        return Err(LegacyAccountError::InvalidAccountData);
                    } else if current.is_some() && stack.len() >= 2 {
                        if current_field.is_some()
                            || is_credential_field(name)
                            || name == "account"
                            || name == "accounts"
                        {
                            return Err(LegacyAccountError::InvalidAccountData);
                        }
                    } else {
                        return Err(LegacyAccountError::MalformedXml);
                    }
                }
                Ok(Event::Text(event)) => {
                    let decoded = event.xml10_content();
                    let value = unescape(&decoded).map_err(|_| LegacyAccountError::MalformedXml)?;
                    if let (Some(fields), Some(field)) = (&mut current, &current_field) {
                        fields
                            .get_mut(field)
                            .ok_or(LegacyAccountError::InvalidAccountData)?
                            .push_str(&value);
                    } else if !value.trim().is_empty() && stack.len() <= 2 {
                        return Err(LegacyAccountError::MalformedXml);
                    }
                }
                Ok(Event::CData(event)) => {
                    let value = event.xml10_content();
                    if let (Some(fields), Some(field)) = (&mut current, &current_field) {
                        fields
                            .get_mut(field)
                            .ok_or(LegacyAccountError::InvalidAccountData)?
                            .push_str(&value);
                    } else if !value.trim().is_empty() && stack.len() <= 2 {
                        return Err(LegacyAccountError::MalformedXml);
                    }
                }
                Ok(Event::GeneralRef(event)) => {
                    let resolved = match event
                        .resolve_char_ref()
                        .map_err(|_| LegacyAccountError::MalformedXml)?
                    {
                        Some(character) => character,
                        None => match event.as_ref() {
                            "amp" => '&',
                            "lt" => '<',
                            "gt" => '>',
                            "apos" => '\'',
                            "quot" => '"',
                            _ => return Err(LegacyAccountError::MalformedXml),
                        },
                    };
                    if let (Some(fields), Some(field)) = (&mut current, &current_field) {
                        fields
                            .get_mut(field)
                            .ok_or(LegacyAccountError::InvalidAccountData)?
                            .push(resolved);
                    } else if stack.len() <= 2 {
                        return Err(LegacyAccountError::MalformedXml);
                    }
                }
                Ok(Event::End(event)) => {
                    let qualified_name = event.name();
                    let name = qualified_name.as_ref();
                    if stack.last().map(String::as_str) != Some(name) {
                        return Err(LegacyAccountError::MalformedXml);
                    }
                    if current_field.as_deref() == Some(name) {
                        current_field = None;
                    } else if name == "account" && stack.len() == 2 {
                        accounts.push(build_account(
                            current
                                .take()
                                .ok_or(LegacyAccountError::InvalidAccountData)?,
                        )?);
                    } else if name == "accounts" && stack.len() == 1 {
                        root_closed = true;
                    }
                    stack.pop();
                }
                Ok(Event::Eof) => break,
                Ok(Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_)) => {}
                Err(_) => return Err(LegacyAccountError::MalformedXml),
            }
        }

        if !stack.is_empty()
            || current.is_some()
            || current_field.is_some()
            || !root_seen
            || !root_closed
        {
            return Err(LegacyAccountError::MalformedXml);
        }

        let mut lookup = HashMap::new();
        for (index, account) in accounts.iter().enumerate() {
            let key = ordinal_ignore_case_key(&account.username);
            if lookup.insert(key, index).is_some() {
                return Err(LegacyAccountError::AmbiguousUsername);
            }
        }
        Ok(Self { accounts, lookup })
    }

    fn account(&self, username: &str) -> Result<&StoredAccount, LegacyAccountError> {
        let key = ordinal_ignore_case_key(username);
        let index = self
            .lookup
            .get(&key)
            .ok_or(LegacyAccountError::UnknownUsername)?;
        self.accounts
            .get(*index)
            .ok_or(LegacyAccountError::InvalidAccountData)
    }
}

fn is_credential_field(name: &str) -> bool {
    matches!(
        name,
        "username" | "password" | "cryptPassword" | "newCryptPassword" | "newSecureCryptPassword"
    )
}

fn build_account(fields: HashMap<String, String>) -> Result<StoredAccount, LegacyAccountError> {
    let username = fields
        .get("username")
        .filter(|value| !value.is_empty())
        .cloned()
        .ok_or(LegacyAccountError::InvalidAccountData)?;
    let credentials = [
        "password",
        "cryptPassword",
        "newCryptPassword",
        "newSecureCryptPassword",
    ]
    .into_iter()
    .filter_map(|field| fields.get(field).map(|value| (field, value)))
    .collect::<Vec<_>>();
    if credentials.len() != 1 {
        return Err(LegacyAccountError::InvalidAccountData);
    }
    let (field, value) = credentials[0];
    let credential = match field {
        "password" => LegacyCredential::Plain(value.clone()),
        "cryptPassword" if valid_digest(value, 16) => LegacyCredential::Md5(value.clone()),
        "newCryptPassword" if valid_digest(value, 20) => LegacyCredential::Sha1(value.clone()),
        "newSecureCryptPassword" if valid_digest(value, 64) => {
            LegacyCredential::Sha512(value.clone())
        }
        _ => return Err(LegacyAccountError::InvalidAccountData),
    };
    Ok(StoredAccount {
        username,
        credential,
    })
}

fn ordinal_ignore_case_key(value: &str) -> String {
    let mut key = String::with_capacity(value.len());
    for character in value.chars() {
        let mut uppercase = character.to_uppercase();
        let first = uppercase.next().unwrap_or(character);
        key.push(if uppercase.next().is_none() {
            first
        } else {
            character
        });
    }
    key
}

impl AccountRepository for LegacyXmlAccountRepository {
    type Account = LegacyAccountIdentity;
    type Error = LegacyAccountError;

    fn authenticate(&self, username: &str, password: &str) -> Result<Self::Account, Self::Error> {
        let account = self.account(username)?;
        let phrase = match &account.credential {
            LegacyCredential::Plain(stored) => {
                return if stored == password {
                    Ok(LegacyAccountIdentity::new(account.username.clone()))
                } else {
                    Err(LegacyAccountError::PasswordMismatch)
                };
            }
            LegacyCredential::Md5(_) => password.to_owned(),
            LegacyCredential::Sha1(_) | LegacyCredential::Sha512(_) => {
                format!("{}{}", account.username, password)
            }
        };
        let input = dotnet_ascii_prefix(&phrase);
        let expected = match &account.credential {
            LegacyCredential::Md5(_) => digest_text(Md5::digest(&input).as_slice()),
            LegacyCredential::Sha1(_) => digest_text(Sha1::digest(&input).as_slice()),
            LegacyCredential::Sha512(_) => digest_text(Sha512::digest(&input).as_slice()),
            LegacyCredential::Plain(_) => unreachable!(),
        };
        let stored = match &account.credential {
            LegacyCredential::Md5(value)
            | LegacyCredential::Sha1(value)
            | LegacyCredential::Sha512(value) => value,
            LegacyCredential::Plain(_) => unreachable!(),
        };
        if expected == *stored {
            Ok(LegacyAccountIdentity::new(account.username.clone()))
        } else {
            Err(LegacyAccountError::PasswordMismatch)
        }
    }
}

impl<R: AccountRepository> ReconnectCredentialVerifier for RepositoryCredentialVerifier<R> {
    type Account = R::Account;
    type Error = R::Error;

    fn verify(&mut self, username: &[u8], password: &[u8]) -> Result<Self::Account, Self::Error> {
        let username = legacy_wire_string(username);
        let password = legacy_wire_string(password);
        self.repository.authenticate(&username, &password)
    }
}

fn legacy_wire_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .map(char::from)
        .collect()
}

fn valid_digest(value: &str, bytes: usize) -> bool {
    value.len() == bytes * 3 - 1
        && value.bytes().enumerate().all(|(index, byte)| {
            if index % 3 == 2 {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit() && !byte.is_ascii_lowercase()
            }
        })
}

fn digest_text(digest: &[u8]) -> String {
    let mut text = String::with_capacity(digest.len() * 3 - 1);
    for (index, byte) in digest.iter().enumerate() {
        if index != 0 {
            text.push('-');
        }
        let _ = write!(text, "{byte:02X}");
    }
    text
}

fn dotnet_ascii_prefix(value: &str) -> Vec<u8> {
    value
        .encode_utf16()
        .take(256)
        .map(|unit| if unit <= 0x7F { unit as u8 } else { b'?' })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{LegacyAccountIdentity, LegacyXmlAccountRepository, RepositoryCredentialVerifier};
    use crate::ReconnectCredentialVerifier;

    static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

    struct XmlFile(PathBuf);

    impl XmlFile {
        fn new(contents: &str) -> Self {
            let sequence = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "rustuo-legacy-accounts-{}-{sequence}.xml",
                std::process::id()
            ));
            fs::write(&path, contents).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for XmlFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn account(username: &str, field: &str, value: &str) -> String {
        format!("<account><username>{username}</username><{field}>{value}</{field}></account>")
    }

    fn verifier(
        xml: &str,
    ) -> (
        XmlFile,
        RepositoryCredentialVerifier<LegacyXmlAccountRepository>,
    ) {
        let file = XmlFile::new(xml);
        let repository = LegacyXmlAccountRepository::open(file.path()).unwrap();
        (file, RepositoryCredentialVerifier::new(repository))
    }

    #[test]
    fn verifies_each_legacy_password_format_and_rejects_wrong_passwords() {
        let xml = format!(
            "<accounts>{}{}{}{}</accounts>",
            account("plain", "password", "secret"),
            account("md5", "cryptPassword", "5E-BE-22-94-EC-D0-E0-F0-8E-AB-76-90-D2-A6-EE-69"),
            account("alice", "newCryptPassword", "BE-2C-E0-75-1D-4A-56-70-9C-4B-7A-4B-A8-1A-CC-6B-F7-54-46-5E"),
            account("bob", "newSecureCryptPassword", "91-3B-46-97-8B-B5-E6-19-C7-18-43-E1-23-DD-89-95-41-E6-6B-9A-93-9E-82-72-6A-07-7D-F3-CB-19-5F-EE-F1-77-2E-48-FB-E3-20-A7-38-3E-62-DB-3A-12-44-04-F2-13-2B-35-4D-C1-D3-DE-4B-78-0B-F0-07-87-FB-EC")
        );
        let (_file, mut repository) = verifier(&xml);

        for username in [b"plain".as_slice(), b"md5", b"alice", b"bob"] {
            repository
                .verify(username, b"secret")
                .unwrap_or_else(|error| {
                    panic!(
                        "credential vector for {} rejected: {error:?}",
                        std::str::from_utf8(username).unwrap()
                    )
                });
        }
        for username in [b"plain".as_slice(), b"md5", b"alice", b"bob"] {
            assert!(repository.verify(username, b"wrong").is_err());
        }
    }

    #[test]
    fn case_insensitive_lookup_uses_original_stored_spelling_for_salted_hash() {
        let xml = account(
            "Alice",
            "newCryptPassword",
            "44-53-11-36-61-AC-8C-A4-20-9C-46-74-3D-9E-96-3C-4A-DF-7A-6D",
        );
        let (_file, mut repository) = verifier(&format!("<accounts>{xml}</accounts>"));

        let identity = repository.verify(b"aLiCe", b"Secret").unwrap();
        assert_eq!(identity.username(), "Alice");
        assert!(repository.verify(b"aLiCe", b"secret").is_err());
    }

    #[test]
    fn applies_dotnet_ascii_replacement_and_256_character_hash_input_limit() {
        let xml = format!(
            "<accounts>{}{}</accounts>",
            account(
                "é",
                "newCryptPassword",
                "17-F0-5C-DE-AF-BF-4B-11-BE-D7-B8-C4-D6-29-12-9F-03-70-75-6D"
            ),
            account(
                &"A".repeat(255),
                "newCryptPassword",
                "A5-9F-EE-DF-A1-F1-9C-C3-FF-6F-64-D7-3D-83-68-59-30-9A-CB-31"
            )
        );
        let (_file, mut repository) = verifier(&xml);

        assert!(repository.verify(&[0xE9], b"secret").is_ok());
        assert!(repository
            .verify("A".repeat(255).as_bytes(), b"xignored-after-256")
            .is_ok());
    }

    #[test]
    fn rejects_unknown_missing_malformed_and_ambiguous_accounts_without_leaking_credentials() {
        let ambiguous = XmlFile::new(&format!(
            "<accounts>{}{}</accounts>",
            account("dup", "password", "secret"),
            account("DUP", "password", "secret")
        ));
        let error = LegacyXmlAccountRepository::open(ambiguous.path())
            .err()
            .unwrap();
        assert_eq!(error, super::LegacyAccountError::AmbiguousUsername);
        assert!(!format!("{error:?}").contains("secret"));

        let (_file, mut repository) = verifier(&format!(
            "<accounts>{}</accounts>",
            account("valid", "password", "secret")
        ));
        assert!(repository.verify(b"unknown", b"secret").is_err());
        let error = repository.verify(b"valid", b"wrong").unwrap_err();
        let rendered = format!("{error:?}");
        assert!(!rendered.contains("secret"));
        assert!(!rendered.contains("5E-BE-22"));

        let malformed = XmlFile::new("<accounts><account>");
        assert!(LegacyXmlAccountRepository::open(malformed.path()).is_err());
        let empty_account = XmlFile::new("<accounts><account/></accounts>");
        assert!(LegacyXmlAccountRepository::open(empty_account.path()).is_err());
        let missing =
            XmlFile::new("<accounts><account><username>missing</username></account></accounts>");
        assert!(LegacyXmlAccountRepository::open(missing.path()).is_err());
    }

    #[test]
    fn rejects_invalid_digest_and_duplicate_credential_fields() {
        for content in [
            account("bad", "cryptPassword", "not a digest"),
            "<account><username>bad</username><password>secret</password><password>other</password></account>".to_owned(),
        ] {
            let file = XmlFile::new(&format!("<accounts>{content}</accounts>"));
            assert!(LegacyXmlAccountRepository::open(file.path()).is_err());
        }
        let missing =
            XmlFile::new("<accounts><account><username>missing</username></account></accounts>");
        assert!(LegacyXmlAccountRepository::open(missing.path()).is_err());
    }

    #[test]
    fn credential_checks_never_rewrite_the_legacy_xml_file() {
        let xml = account("plain", "password", "secret");
        let (file, mut repository) = verifier(&format!("<accounts>{xml}</accounts>"));
        let original = fs::read(file.path()).unwrap();

        assert!(repository.verify(b"plain", b"secret").is_ok());
        assert!(repository.verify(b"plain", b"wrong").is_err());
        assert_eq!(fs::read(file.path()).unwrap(), original);
    }

    #[test]
    fn preserves_plain_password_whitespace_from_xml_text() {
        let (_file, mut repository) = verifier(&format!(
            "<accounts>{}</accounts>",
            account("space", "password", " secret ")
        ));

        assert!(repository.verify(b"space", b" secret ").is_ok());
        assert!(repository.verify(b"space", b"secret").is_err());
    }

    #[test]
    fn loads_servuo_accounts_with_nested_noncredential_metadata() {
        let xml = r#"<accounts count="1"><account serial="7">
            <username>Alice</username><password>secret</password>
            <created>2020-01-02T03:04:05Z</created><lastLogin>2021-02-03T04:05:06Z</lastLogin>
            <totalGameTime>12.00:00:00</totalGameTime><chars><char index="0">0x400001</char><char index="1">0x400002</char></chars>
            <comments><comment author="staff">reviewed</comment></comments>
            <metadata><empty/><custom key="value">kept only as ignored metadata</custom></metadata>
        </account></accounts>"#;
        let (file, mut repository) = verifier(xml);
        let original = fs::read(file.path()).unwrap();

        assert_eq!(
            repository.verify(b"Alice", b"secret").unwrap().username(),
            "Alice"
        );
        assert_eq!(fs::read(file.path()).unwrap(), original);
    }

    #[test]
    fn wire_fields_use_legacy_latin1_byte_to_character_mapping() {
        let (_file, mut repository) = verifier(
            "<accounts><account><username>é</username><password>é</password></account></accounts>",
        );

        assert_eq!(repository.verify(&[0xE9], &[0xE9]).unwrap().username(), "é");
        assert!(repository.verify("é".as_bytes(), "é".as_bytes()).is_err());
    }

    #[test]
    fn repository_contract_is_implementable_without_private_credential_types() {
        mod external_style {
            use super::super::{AccountRepository, LegacyAccountError, LegacyAccountIdentity};

            pub struct FakeRepository;

            impl AccountRepository for FakeRepository {
                type Account = LegacyAccountIdentity;
                type Error = LegacyAccountError;

                fn authenticate(
                    &self,
                    username: &str,
                    password: &str,
                ) -> Result<Self::Account, Self::Error> {
                    if username == "fixture" && password == "secret" {
                        Ok(LegacyAccountIdentity::new("fixture"))
                    } else {
                        Err(LegacyAccountError::PasswordMismatch)
                    }
                }
            }
        }

        let mut verifier = RepositoryCredentialVerifier::new(external_style::FakeRepository);
        assert_eq!(
            verifier.verify(b"fixture", b"secret").unwrap().username(),
            "fixture"
        );
    }

    #[test]
    fn rejects_nonwhitespace_text_outside_structural_account_fields() {
        for xml in [
            "<accounts>unexpected<account><username>user</username><password>pass</password></account></accounts>",
            "<accounts><account><username>user</username><password>pass</password>unexpected</account></accounts>",
        ] {
            let file = XmlFile::new(xml);
            assert!(LegacyXmlAccountRepository::open(file.path()).is_err());
        }
    }

    #[test]
    fn adapter_returns_a_name_based_identity_not_a_numeric_account_id() {
        let (_file, mut repository) = verifier(&format!(
            "<accounts>{}</accounts>",
            account("Mira", "password", "secret")
        ));

        assert_eq!(
            repository.verify(b"mira", b"secret"),
            Ok(LegacyAccountIdentity::new("Mira"))
        );
    }
}
