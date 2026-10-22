//! Reports and expenses commands.

use tauri::State;

use crate::commands::AppState;
use crate::db::{expenses, reports};
use crate::error::CommandResult;

#[tauri::command]
pub async fn get_receivables(
    state: State<'_, AppState>,
) -> CommandResult<reports::Receivables> {
    Ok(reports::receivables(&state.pool).await?)
}

#[tauri::command]
pub async fn get_cash_flow(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> CommandResult<reports::CashFlow> {
    Ok(reports::cash_flow(&state.pool, &from, &to).await?)
}

#[tauri::command]
pub async fn list_expense_categories(
    state: State<'_, AppState>,
) -> CommandResult<Vec<expenses::ExpenseCategory>> {
    Ok(expenses::categories(&state.pool).await?)
}

#[tauri::command]
pub async fn list_expenses(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> CommandResult<Vec<expenses::Expense>> {
    Ok(expenses::list(&state.pool, &from, &to).await?)
}

#[tauri::command]
pub async fn create_expense(
    state: State<'_, AppState>,
    input: expenses::ExpenseInput,
) -> CommandResult<expenses::Expense> {
    Ok(expenses::create(&state.pool, &input).await?)
}

#[tauri::command]
pub async fn delete_expense(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    Ok(expenses::soft_delete(&state.pool, id).await?)
}
