use crate::tokens::TokenVerificationKey;

#[derive(Clone)]
pub struct InternalAuthService {
    viewer_event_token_verifying_key: TokenVerificationKey,
}

impl InternalAuthService {
    pub fn new(
        viewer_event_token_verifying_key: TokenVerificationKey,
    ) -> InternalAuthService {
        InternalAuthService {
            viewer_event_token_verifying_key,
        }
    }

    pub async fn viewer_event_token_pub_key(&self) -> &TokenVerificationKey {
        &self.viewer_event_token_verifying_key
    }
}
