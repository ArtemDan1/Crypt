use crate::commands::lifecycle::{VaultStatus, header_vault_id, install};
use crate::error::{ApiError, ApiResult};
use crate::state::AppState;
use crate::unlock::UnlockMethods;
use tauri::{AppHandle, Manager};

fn locked_id(st: &AppState) -> ApiResult<String> {
    header_vault_id(st).ok_or_else(|| ApiError::new("NotAVault", "Хранилище не найдено"))
}

fn with_open<T>(st: &AppState, f: impl FnOnce(&crypt_core::Vault) -> ApiResult<T>) -> ApiResult<T> {
    let g = st.vault_guard();
    let v = g.as_ref().ok_or_else(|| ApiError::new("Locked", "Хранилище заблокировано"))?;
    f(v)
}

/// Run blocking work (Touch ID prompt, Argon2) off the async runtime.
async fn blocking<T: Send + 'static>(
    app: AppHandle,
    f: impl FnOnce(&AppState) -> ApiResult<T> + Send + 'static,
) -> ApiResult<T> {
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<std::sync::Arc<AppState>>()))
        .await
        .map_err(|_| ApiError::new("Internal", "Внутренняя ошибка"))?
}

#[tauri::command]
pub async fn unlock_methods(app: AppHandle) -> ApiResult<UnlockMethods> {
    blocking(app, |st| st.unlocker().methods(&locked_id(st)?)).await
}

#[tauri::command]
pub async fn enable_biometric(app: AppHandle) -> ApiResult<()> {
    blocking(app, |st| with_open(st, |v| st.unlocker().enable_biometric(v))).await
}

#[tauri::command]
pub async fn disable_biometric(app: AppHandle) -> ApiResult<()> {
    blocking(app, |st| st.unlocker().disable_biometric(&locked_id(st)?)).await
}

#[tauri::command]
pub async fn unlock_biometric(app: AppHandle) -> ApiResult<VaultStatus> {
    blocking(app, |st| {
        let id = locked_id(st)?;
        let v = st.unlocker().unlock_biometric(&st.vault_dir, &id)?;
        Ok(install(st, v))
    })
    .await
}

#[tauri::command]
pub async fn set_pin(app: AppHandle, pin: String) -> ApiResult<()> {
    blocking(app, move |st| with_open(st, |v| st.unlocker().set_pin(v, &pin))).await
}

#[tauri::command]
pub async fn remove_pin(app: AppHandle) -> ApiResult<()> {
    blocking(app, |st| st.unlocker().remove_pin(&locked_id(st)?)).await
}

#[tauri::command]
pub async fn unlock_pin(app: AppHandle, pin: String) -> ApiResult<VaultStatus> {
    blocking(app, move |st| {
        let id = locked_id(st)?;
        let v = st.unlocker().unlock_pin(&st.vault_dir, &id, &pin)?;
        Ok(install(st, v))
    })
    .await
}

/// Needs fresh presence proof even while unlocked.
#[tauri::command]
pub async fn reveal_seed(app: AppHandle, pin: Option<String>) -> ApiResult<Vec<String>> {
    blocking(app, move |st| {
        st.unlocker().confirm_presence(&locked_id(st)?, pin.as_deref())?;
        with_open(st, |v| {
            Ok(v.reveal_seed()?.words().into_iter().map(str::to_owned).collect())
        })
    })
    .await
}
