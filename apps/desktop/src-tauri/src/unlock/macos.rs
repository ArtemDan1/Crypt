use super::authenticator::{AuthError, Authenticator};
use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_foundation::{NSError, NSString};
use objc2_local_authentication::{LAContext, LAError, LAPolicy};
use std::sync::mpsc;

/// Touch ID with fallback to the Mac login password (`.deviceOwnerAuthentication`).
/// Needs no entitlements, so it works with a self-signed build.
pub struct MacAuthenticator;

impl Authenticator for MacAuthenticator {
    fn available(&self) -> bool {
        unsafe {
            let ctx = LAContext::new();
            ctx.canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthentication).is_ok()
        }
    }

    fn authenticate(&self, reason: &str) -> Result<(), AuthError> {
        let (tx, rx) = mpsc::channel::<Result<(), AuthError>>();
        unsafe {
            let ctx = LAContext::new();
            if ctx.canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthentication).is_err() {
                return Err(AuthError::Unavailable);
            }
            let reply = RcBlock::new(move |ok: Bool, err: *mut NSError| {
                let res = if ok.as_bool() {
                    Ok(())
                } else if err.is_null() {
                    Err(AuthError::Failed)
                } else {
                    let code = (*err).code();
                    if code == LAError::UserCancel.0
                        || code == LAError::AppCancel.0
                        || code == LAError::SystemCancel.0
                    {
                        Err(AuthError::Cancelled)
                    } else if code == LAError::BiometryNotAvailable.0
                        || code == LAError::PasscodeNotSet.0
                        || code == LAError::NotInteractive.0
                    {
                        Err(AuthError::Unavailable)
                    } else {
                        Err(AuthError::Failed)
                    }
                };
                let _ = tx.send(res);
            });
            ctx.evaluatePolicy_localizedReason_reply(
                LAPolicy::DeviceOwnerAuthentication,
                &NSString::from_str(reason),
                &reply,
            );
        }
        rx.recv().unwrap_or(Err(AuthError::Failed))
    }
}
