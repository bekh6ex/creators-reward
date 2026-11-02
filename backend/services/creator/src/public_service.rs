use std::sync::Arc;

use futures::lock::Mutex;
use rand::RngCore;
use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::repository::{Account, Repository};

/// Same minimum the portal's register form asks for.
pub const MIN_PASSWORD_LENGTH: usize = 8;
const MAX_TITLE_LENGTH: usize = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CreatorId(Uuid);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Creator {
    pub id: CreatorId,
    pub email: String,
}

/// The portal's `Website` type, field for field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Website {
    pub id: String,
    /// Lower-case host name without scheme, port or path.
    pub domain: String,
    pub title: String,
    /// RFC 3339, UTC.
    pub added_at: String,
    pub status: VerificationStatus,
    /// What the owner will put in the verification meta tag once ownership
    /// checks exist. Random and per site.
    pub token: String,
    /// Always `None` for now: nothing is checked.
    pub last_checked_at: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    /// Connected without a check, for now.
    Verified,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RegisterError {
    #[error("email already registered")]
    EmailTaken,
    #[error("not an email address, or a password shorter than {MIN_PASSWORD_LENGTH}")]
    Malformed,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum LoginError {
    #[error("wrong email or password")]
    InvalidCredentials,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum AddWebsiteError {
    #[error("not a domain name, or a title that is too long")]
    Malformed,
    /// Some creator, possibly the same one, already added this domain.
    #[error("domain already registered")]
    Taken,
}

#[derive(Clone, Default)]
pub struct CreatorService {
    repository: Arc<Mutex<Repository>>,
}

impl CreatorService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates the account and logs it in: returns a session id.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
    ) -> Result<String, RegisterError> {
        let email = normalise_email(email).ok_or(RegisterError::Malformed)?;
        if password.chars().count() < MIN_PASSWORD_LENGTH {
            return Err(RegisterError::Malformed);
        }
        let session = {
            let mut repository = self.repository.lock().await;
            if repository.by_email.contains_key(&email) {
                return Err(RegisterError::EmailTaken);
            }
            let id = CreatorId(Uuid::new_v4());
            repository.by_email.insert(email.clone(), id);
            repository.accounts.insert(
                id,
                Account {
                    email,
                    password: password.to_string(),
                },
            );
            let session = new_session(&mut repository, id);
            drop(repository);
            session
        };
        Ok(session)
    }

    /// Returns a session id.
    pub async fn login(&self, email: &str, password: &str) -> Result<String, LoginError> {
        let email = normalise_email(email).ok_or(LoginError::InvalidCredentials)?;
        let session = {
            let mut repository = self.repository.lock().await;
            let id = *repository
                .by_email
                .get(&email)
                .ok_or(LoginError::InvalidCredentials)?;
            let matches = repository
                .accounts
                .get(&id)
                .is_some_and(|account| account.password == password);
            if !matches {
                return Err(LoginError::InvalidCredentials);
            }
            let session = new_session(&mut repository, id);
            drop(repository);
            session
        };
        Ok(session)
    }

    pub async fn logout(&self, session: &str) {
        self.repository
            .lock()
            .await
            .sessions
            .remove(session);
    }

    pub async fn session(&self, session: &str) -> Option<Creator> {
        let (id, email) = {
            let repository = self.repository.lock().await;
            let id = *repository.sessions.get(session)?;
            let account = repository.accounts.get(&id)?;
            let email = account.email.clone();
            drop(repository);
            (id, email)
        };
        Some(Creator { id, email })
    }

    /// The creator's websites, oldest first.
    pub async fn websites(&self, creator: CreatorId) -> Vec<Website> {
        let repository = self.repository.lock().await;
        repository
            .websites
            .iter()
            .filter(|(owner, _)| *owner == creator)
            .map(|(_, website)| website.clone())
            .collect()
    }

    /// `domain` must already be normalised (the portal strips scheme, `www.`
    /// and path); an empty `title` defaults to the domain.
    pub async fn add_website(
        &self,
        creator: CreatorId,
        domain: &str,
        title: &str,
    ) -> Result<Website, AddWebsiteError> {
        let domain = domain.trim();
        if !is_domain(domain) {
            return Err(AddWebsiteError::Malformed);
        }
        let title = match title.trim() {
            "" => domain,
            title if title.chars().count() > MAX_TITLE_LENGTH => {
                return Err(AddWebsiteError::Malformed);
            }
            title => title,
        };
        let website = {
            let mut repository = self.repository.lock().await;
            if repository
                .websites
                .iter()
                .any(|(_, website)| website.domain == domain)
            {
                return Err(AddWebsiteError::Taken);
            }
            let website = Website {
                id: Uuid::new_v4().to_string(),
                domain: domain.to_string(),
                title: title.to_string(),
                added_at: now_rfc3339(),
                status: VerificationStatus::Verified,
                token: format!("crv-{}", random_hex::<16>()),
                last_checked_at: None,
            };
            repository
                .websites
                .push((creator, website.clone()));
            drop(repository);
            website
        };
        Ok(website)
    }
}

fn new_session(repository: &mut Repository, creator: CreatorId) -> String {
    let session = random_hex::<32>();
    repository
        .sessions
        .insert(session.clone(), creator);
    session
}

fn random_hex<const N: usize>() -> String {
    let mut bytes = [0u8; N];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[expect(
    clippy::disallowed_methods,
    reason = "creator portal site metadata (added_at), not a reader event timestamp"
)]
#[expect(
    clippy::expect_used,
    reason = "nanosecond 0 is valid for any OffsetDateTime and RFC 3339 formatting a valid datetime cannot fail"
)]
fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .replace_nanosecond(0)
        .expect("0 is a valid nanosecond")
        .format(&Rfc3339)
        .expect("the current time formats as RFC 3339")
}

fn normalise_email(raw: &str) -> Option<String> {
    let email = raw.trim().to_lowercase();
    let (local, domain) = email.split_once('@')?;
    let valid = !local.is_empty()
        && !domain.contains('@')
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && !email.chars().any(char::is_whitespace);
    valid.then_some(email)
}

/// Lower-case host name with at least two labels, the rule the portal's
/// `validateDomain` applies. No port, scheme or path.
fn is_domain(domain: &str) -> bool {
    let labels: Vec<&str> = domain.split('.').collect();
    domain.len() <= 253
        && labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use googletest::prelude::*;
    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn service() -> CreatorService {
        CreatorService::new()
    }

    async fn register_owner(service: &CreatorService) -> CreatorId {
        let session = service
            .register("owner@example.org", "password")
            .await
            .unwrap();
        service.session(&session).await.unwrap().id
    }

    #[gtest]
    #[rstest]
    fn registering_logs_in_and_the_email_is_taken_afterwards(service: CreatorService) {
        block_on(async {
            let session = service
                .register(" Owner@Example.org ", "correct horse")
                .await
                .unwrap();
            let creator = service.session(&session).await.unwrap();
            expect_that!(creator.email, eq("owner@example.org"));

            expect_that!(
                service
                    .register("owner@example.org", "another one")
                    .await,
                err(eq(&RegisterError::EmailTaken))
            );
        });
    }

    #[gtest]
    #[rstest]
    #[case::no_at_sign("not-an-email", "long enough")]
    #[case::no_dot_in_the_domain("a@b", "long enough")]
    #[case::whitespace("two words@example.org", "long enough")]
    #[case::short_password("owner@example.org", "short")]
    fn registration_needs_an_email_and_a_long_enough_password(
        service: CreatorService,
        #[case] email: &str,
        #[case] password: &str,
    ) {
        expect_that!(
            block_on(service.register(email, password)),
            err(eq(&RegisterError::Malformed))
        );
    }

    #[gtest]
    #[rstest]
    fn login_checks_the_password_and_logout_ends_only_that_session(
        service: CreatorService,
    ) {
        block_on(async {
            let first = service
                .register("owner@example.org", "correct horse")
                .await
                .unwrap();
            expect_that!(
                service
                    .login("owner@example.org", "wrong horse")
                    .await,
                err(eq(&LoginError::InvalidCredentials))
            );
            expect_that!(
                service
                    .login("nobody@example.org", "correct horse")
                    .await,
                err(eq(&LoginError::InvalidCredentials))
            );
            let second = service
                .login("OWNER@example.org", "correct horse")
                .await
                .unwrap();
            expect_that!(first, ne(&second));

            service.logout(&first).await;
            expect_that!(service.session(&first).await, none());
            expect_that!(service.session(&second).await, some(anything()));
        });
    }

    #[gtest]
    #[rstest]
    fn a_website_is_connected_at_once_and_belongs_to_one_creator(
        service: CreatorService,
    ) {
        block_on(async {
            let alice = service
                .register("alice@example.org", "password1")
                .await
                .unwrap();
            let alice = service.session(&alice).await.unwrap().id;
            let bob = service
                .register("bob@example.org", "password2")
                .await
                .unwrap();
            let bob = service.session(&bob).await.unwrap().id;

            let website = service
                .add_website(alice, "website1.crlocal", "  ")
                .await
                .unwrap();
            expect_that!(website.title, eq("website1.crlocal"));
            expect_that!(website.status, eq(VerificationStatus::Verified));
            expect_that!(website.last_checked_at, none());
            expect_that!(
                OffsetDateTime::parse(&website.added_at, &Rfc3339),
                ok(anything())
            );
            expect_that!(website.token, char_count(eq(36)));

            expect_that!(
                service.websites(alice).await,
                elements_are![eq(&website)]
            );
            expect_that!(service.websites(bob).await, is_empty());
            for owner in [alice, bob] {
                expect_that!(
                    service
                        .add_website(owner, "website1.crlocal", "")
                        .await,
                    err(eq(&AddWebsiteError::Taken))
                );
            }

            let second = service
                .add_website(alice, "website2.crlocal", "Second site")
                .await
                .unwrap();
            expect_that!(second.title, eq("Second site"));
            let domains: Vec<_> = service
                .websites(alice)
                .await
                .into_iter()
                .map(|website| website.domain)
                .collect();
            expect_that!(
                domains,
                elements_are![eq("website1.crlocal"), eq("website2.crlocal")]
            );
        });
    }

    #[gtest]
    #[rstest]
    #[case::subdomain_with_hyphen("news-1.example.org", None)]
    #[case::empty("", Some(AddWebsiteError::Malformed))]
    #[case::single_label("localhost", Some(AddWebsiteError::Malformed))]
    #[case::upper_case("Example.org", Some(AddWebsiteError::Malformed))]
    #[case::scheme("https://example.org", Some(AddWebsiteError::Malformed))]
    #[case::path("example.org/news", Some(AddWebsiteError::Malformed))]
    #[case::port("example.org:8443", Some(AddWebsiteError::Malformed))]
    #[case::leading_hyphen("-example.org", Some(AddWebsiteError::Malformed))]
    #[case::empty_label("example..org", Some(AddWebsiteError::Malformed))]
    fn only_bare_domains_are_accepted(
        service: CreatorService,
        #[case] domain: &str,
        #[case] error: Option<AddWebsiteError>,
    ) {
        block_on(async {
            let owner = register_owner(&service).await;
            expect_that!(
                service.add_website(owner, domain, "").await.err(),
                eq(&error)
            );
        });
    }

    #[gtest]
    #[rstest]
    fn the_title_has_a_maximum_length(service: CreatorService) {
        block_on(async {
            let owner = register_owner(&service).await;
            expect_that!(
                service
                    .add_website(owner, "example.org", &"x".repeat(MAX_TITLE_LENGTH + 1))
                    .await,
                err(eq(&AddWebsiteError::Malformed))
            );
        });
    }
}
