mod commands;
mod db;
mod errors;
mod models;
mod services;

use serde::Serialize;
use tauri::{Manager, State};

use commands::{
    auth::{
        create_user, deactivate_user, get_current_user, list_users, login, logout, update_user,
        SessionStore,
    },
    force_error, ping,
    inventory::{get_item_movements, adjust_stock},
    items::{
        create_category, create_item, delete_category, delete_item, get_item_by_barcode,
        list_categories, list_items, update_item,
    },
    print::{
        generate_invoice_pdf, get_invoice_print_data,
        get_label_printer_list, get_purchase_print_data, get_return_print_data, list_printers,
        open_whatsapp_with_invoice, read_logo_base64, save_logo, delete_logo,
    },
    reports::{
        report_daily_sales, report_low_stock, report_sales_by_period, report_top_items,
    },
    suppliers::list_suppliers,
    customers::list_customers,
    sales::{
        create_return, create_sale_invoice, get_invoice_detail, get_invoice_stats, list_invoices,
        search_items, list_debts,
    },
    purchases::{
        create_purchase_invoice, get_item_purchase_history, get_purchase_detail, get_purchase_stats,
        list_purchases, update_purchase_invoice, search_items_for_purchase,
    },
    backup::{list_backups, restore_backup, trigger_backup},
    sessions::{
        close_session, get_active_session, get_session_sales_total_millieme, open_session,
    },
    settings::{
        get_db_file_size,
        get_setting,
        get_settings,
        update_settings,
        vacuum_database,
        is_first_launch,
        complete_setup,
        seed_sample_data,
    },
    items::import_items_csv,
};
use db::DbPool;
use errors::AppError;
use services::backup::BackupService;

#[derive(Debug, Serialize)]
struct DbInfo {
    journal_mode: String,
}

#[tauri::command]
async fn get_db_info(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
) -> Result<DbInfo, AppError> {
    commands::guard::require_role(&sessions, &pool, token, commands::guard::ANY_ROLE).await?;

    let row: (String,) = sqlx::query_as("PRAGMA journal_mode;")
        .fetch_one(&*pool)
        .await?;

    Ok(DbInfo {
        journal_mode: row.0,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let pool = tauri::async_runtime::block_on(db::get_pool());
            let backup_service = BackupService::new();
            let worker_pool = pool.clone();
            app.manage(pool);
            app.manage(backup_service.clone());
            app.manage(SessionStore::default());
            let backup_worker = backup_service.clone();

            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(4 * 60 * 60)).await;

                    if let Err(error) = backup_worker
                        .create_backup_with_checkpoint(&worker_pool)
                        .await
                    {
                        eprintln!("Automatic periodic backup failed: {error:?}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_db_info,
            login,
            logout,
            get_current_user,
            list_users,
            create_user,
            update_user,
            deactivate_user,
            ping,
            force_error,
            list_items,
            search_items,
            get_item_by_barcode,
            create_item,
            update_item,
            delete_item,
            get_item_movements,
            adjust_stock,
            list_categories,
            create_category,
            delete_category,
            get_active_session,
            open_session,
            close_session,
            get_session_sales_total_millieme,
            trigger_backup,
            list_backups,
            restore_backup,
            create_sale_invoice,
            create_return,
            list_invoices,
            list_debts,
            get_invoice_detail,
            get_invoice_stats,
            list_purchases,
            get_purchase_detail,
            create_purchase_invoice,
            update_purchase_invoice,
            get_purchase_stats,
            get_item_purchase_history,
            search_items_for_purchase,
            report_daily_sales,
            report_sales_by_period,
            report_top_items,
            report_low_stock,
            list_suppliers,
            list_customers,
            generate_invoice_pdf,
            get_invoice_print_data,
            get_purchase_print_data,
            get_return_print_data,
            open_whatsapp_with_invoice,
            read_logo_base64,
            save_logo,
            delete_logo,
            list_printers,
            get_label_printer_list,
            import_items_csv,
            get_settings,
            update_settings,
            get_setting,
            vacuum_database,
            get_db_file_size,
            is_first_launch,
            complete_setup,
            seed_sample_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
