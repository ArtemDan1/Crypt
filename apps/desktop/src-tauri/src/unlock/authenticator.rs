/// Proof that the owner is present: Touch ID or the Mac login password.
pub trait Authenticator: Send + Sync {
    fn authenticate(&self, reason: &str) -> Result<(), AuthError>;
    fn available(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthError {
    Cancelled,
    Unavailable,
    Failed,
}

impl From<AuthError> for crate::error::ApiError {
    fn from(e: AuthError) -> Self {
        match e {
            AuthError::Cancelled => Self::new("AuthCancelled", "Подтверждение отменено"),
            AuthError::Unavailable => Self::new("AuthUnavailable", "Touch ID и пароль Mac сейчас недоступны"),
            AuthError::Failed => Self::new("AuthFailed", "Не удалось подтвердить личность"),
        }
    }
}

/// Test double with a fixed answer.
pub struct FakeAuthenticator(Result<(), AuthError>);
impl FakeAuthenticator {
    pub fn ok() -> Self {
        Self(Ok(()))
    }
    pub fn err(e: AuthError) -> Self {
        Self(Err(e))
    }
}
impl Authenticator for FakeAuthenticator {
    fn authenticate(&self, _: &str) -> Result<(), AuthError> {
        self.0
    }
    fn available(&self) -> bool {
        self.0 != Err(AuthError::Unavailable)
    }
}
