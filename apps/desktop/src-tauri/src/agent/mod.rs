pub mod protocol;

#[cfg(test)]
mod tests;

pub mod cache;
pub mod policy;
pub mod server;

use crate::state::AppState;
use serde::Serialize;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub running: bool,
    pub socket_path: String,
    pub identities: usize,
    pub error: Option<String>,
}
#[tauri::command]
pub async fn agent_status(
    st: tauri::State<'_, std::sync::Arc<AppState>>,
) -> crate::error::ApiResult<AgentStatus> {
    Ok(AgentStatus {
        running: st.agent.running(),
        socket_path: st.socket_path().display().to_string(),
        identities: policy::identities(&st).len(),
        error: st
            .agent_error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone(),
    })
}

pub fn set_running(st: std::sync::Arc<AppState>, enabled: bool) -> crate::error::ApiResult<()> {
    let result = if enabled {
        st.agent.start(st.clone(), st.socket_path())
    } else {
        st.agent.stop();
        Ok(())
    };
    let error = result
        .err()
        .map(|_| "Не удалось запустить ssh-agent; проверьте путь к сокету".to_string());
    *st.agent_error.lock().unwrap_or_else(|e| e.into_inner()) = error.clone();
    st.emit("agent://changed", ());
    error.map_or(Ok(()), |message| {
        Err(crate::error::ApiError::new("Agent", message))
    })
}
pub fn start_configured(st: std::sync::Arc<AppState>) {
    let enabled = crate::commands::lifecycle::get_settings(&st).agent_enabled;
    if enabled {
        let _ = set_running(st, true);
    }
}
