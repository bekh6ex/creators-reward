//! Gateway-side error mapping.
//!
//! Services return typed error enums; the gateway turns each variant into an
//! HTTP status. Every mapping is an exhaustive match, so a new service variant
//! is a compile error until the gateway decides what status it produces.

use axum::http::StatusCode;

/// The HTTP status a service error variant produces.
///
/// Implemented once per service error enum, with an exhaustive match and no
/// catch-all arm, so adding a variant breaks the gateway build until it is
/// mapped.
pub(crate) trait HttpError {
    fn status(&self) -> StatusCode;
}

impl HttpError for auth::public_service::RegisterError {
    fn status(&self) -> StatusCode {
        match self {
            Self::EmailTaken => StatusCode::CONFLICT,
            Self::Malformed => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl HttpError for auth::public_service::LoginError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Malformed => StatusCode::UNPROCESSABLE_ENTITY,
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl HttpError for billing::service::CurrentPeriodKeyError {
    fn status(&self) -> StatusCode {
        match self {
            Self => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl HttpError for billing::admin_service::AddKeyError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Ended(_) => StatusCode::CONFLICT,
            Self::Rejected(rejected) => rejected.status(),
        }
    }
}

impl HttpError for billing::admin_service::KeyRejected {
    fn status(&self) -> StatusCode {
        match self {
            Self::Exists(_) | Self::Closed(_) => StatusCode::CONFLICT,
        }
    }
}

impl HttpError for billing::admin_service::SettleError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotAwaiting(_) => StatusCode::CONFLICT,
            Self::MoreThanCharged { .. } => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl HttpError for creator::public_service::RegisterError {
    fn status(&self) -> StatusCode {
        match self {
            Self::EmailTaken => StatusCode::CONFLICT,
            Self::Malformed => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }
}

impl HttpError for creator::public_service::LoginError {
    fn status(&self) -> StatusCode {
        match self {
            Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
        }
    }
}

impl HttpError for creator::public_service::AddWebsiteError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Malformed => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Taken => StatusCode::CONFLICT,
        }
    }
}

impl HttpError for web_viewer_events::service::PublicKeyError {
    fn status(&self) -> StatusCode {
        match self {
            Self => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl<E> HttpError for web_viewer_events::service::Error<E> {
    fn status(&self) -> StatusCode {
        match self {
            Self::SendError(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use googletest::prelude::*;
    use rstest::rstest;

    use super::HttpError;

    #[gtest]
    #[rstest]
    #[case::email_taken(
        auth::public_service::RegisterError::EmailTaken,
        StatusCode::CONFLICT
    )]
    #[case::malformed(
        auth::public_service::RegisterError::Malformed,
        StatusCode::UNPROCESSABLE_ENTITY
    )]
    #[case::unavailable(
        auth::public_service::RegisterError::Unavailable,
        StatusCode::SERVICE_UNAVAILABLE
    )]
    fn auth_register_error_status(
        #[case] error: auth::public_service::RegisterError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::malformed(
        auth::public_service::LoginError::Malformed,
        StatusCode::UNPROCESSABLE_ENTITY
    )]
    #[case::invalid_credentials(
        auth::public_service::LoginError::InvalidCredentials,
        StatusCode::UNAUTHORIZED
    )]
    #[case::unavailable(
        auth::public_service::LoginError::Unavailable,
        StatusCode::SERVICE_UNAVAILABLE
    )]
    fn auth_login_error_status(
        #[case] error: auth::public_service::LoginError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::no_key(
        billing::service::CurrentPeriodKeyError,
        StatusCode::SERVICE_UNAVAILABLE
    )]
    fn billing_current_period_key_error_status(
        #[case] error: billing::service::CurrentPeriodKeyError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::ended(
        billing::admin_service::AddKeyError::Ended(billing::period::PeriodId::new(1)),
        StatusCode::CONFLICT
    )]
    #[case::exists(
        billing::admin_service::AddKeyError::Rejected(
            billing::admin_service::KeyRejected::Exists(billing::period::PeriodId::new(
                1
            ))
        ),
        StatusCode::CONFLICT
    )]
    #[case::closed(
        billing::admin_service::AddKeyError::Rejected(
            billing::admin_service::KeyRejected::Closed(billing::period::PeriodId::new(
                1
            ))
        ),
        StatusCode::CONFLICT
    )]
    fn billing_add_key_error_status(
        #[case] error: billing::admin_service::AddKeyError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::not_awaiting(
        billing::admin_service::SettleError::NotAwaiting(
            billing::period::PeriodId::new(1)
        ),
        StatusCode::CONFLICT
    )]
    #[case::more_than_charged(billing::admin_service::SettleError::MoreThanCharged { paid: billing::EuroCents(10), charged: billing::EuroCents(5) }, StatusCode::UNPROCESSABLE_ENTITY)]
    fn billing_settle_error_status(
        #[case] error: billing::admin_service::SettleError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::email_taken(
        creator::public_service::RegisterError::EmailTaken,
        StatusCode::CONFLICT
    )]
    #[case::malformed(
        creator::public_service::RegisterError::Malformed,
        StatusCode::UNPROCESSABLE_ENTITY
    )]
    fn creator_register_error_status(
        #[case] error: creator::public_service::RegisterError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::invalid_credentials(
        creator::public_service::LoginError::InvalidCredentials,
        StatusCode::UNAUTHORIZED
    )]
    fn creator_login_error_status(
        #[case] error: creator::public_service::LoginError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::malformed(
        creator::public_service::AddWebsiteError::Malformed,
        StatusCode::UNPROCESSABLE_ENTITY
    )]
    #[case::taken(
        creator::public_service::AddWebsiteError::Taken,
        StatusCode::CONFLICT
    )]
    fn creator_add_website_error_status(
        #[case] error: creator::public_service::AddWebsiteError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::not_registered(
        web_viewer_events::service::PublicKeyError,
        StatusCode::SERVICE_UNAVAILABLE
    )]
    fn web_events_public_key_error_status(
        #[case] error: web_viewer_events::service::PublicKeyError,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }

    #[gtest]
    #[rstest]
    #[case::send_error(web_viewer_events::service::Error::SendError(()), StatusCode::SERVICE_UNAVAILABLE)]
    fn web_events_error_status(
        #[case] error: web_viewer_events::service::Error<()>,
        #[case] expected: StatusCode,
    ) {
        expect_that!(error.status(), eq(expected));
    }
}
