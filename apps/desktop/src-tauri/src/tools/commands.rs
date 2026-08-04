use tauri::State;

use super::types::*;
use super::ToolState;

#[tauri::command]
pub fn list_tools(state: State<'_, ToolState>) -> Result<ToolListResponse, String> {
    Ok(state.service.list_tools())
}

#[tauri::command]
pub async fn web_search_tool(
    state: State<'_, ToolState>,
    request: WebSearchRequest,
) -> Result<ToolRunResponse, String> {
    state.service.web_search(request).await
}

#[tauri::command]
pub async fn fetch_page_tool(
    state: State<'_, ToolState>,
    request: WebFetchRequest,
) -> Result<ToolRunResponse, String> {
    state.service.fetch_page(request).await
}

#[tauri::command]
pub fn read_file_tool(
    state: State<'_, ToolState>,
    request: FileReadRequest,
) -> Result<ToolRunResponse, String> {
    state.service.read_file(request)
}

#[tauri::command]
pub fn git_status_tool(
    state: State<'_, ToolState>,
    request: GitReadRequest,
) -> Result<ToolRunResponse, String> {
    state.service.git_status(request)
}

#[tauri::command]
pub fn git_diff_tool(
    state: State<'_, ToolState>,
    request: GitReadRequest,
) -> Result<ToolRunResponse, String> {
    state.service.git_diff(request)
}

#[tauri::command]
pub fn commit_message_tool(
    state: State<'_, ToolState>,
    request: GitReadRequest,
) -> Result<ToolRunResponse, String> {
    state.service.commit_message(request)
}

#[tauri::command]
pub fn code_task_tool(
    state: State<'_, ToolState>,
    request: CodeTaskRequest,
) -> Result<ToolRunResponse, String> {
    state.service.code_task(request)
}

#[tauri::command]
pub fn day_planner_tool(
    state: State<'_, ToolState>,
    request: DayPlannerRequest,
) -> Result<ToolRunResponse, String> {
    state.service.day_planner(request)
}
