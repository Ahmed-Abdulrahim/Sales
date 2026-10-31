use std::collections::HashMap;

use tauri::State;

use crate::{
    commands::purchases::get_purchase_detail_impl,
    commands::sales::fetch_invoice_detail,
    commands::sales::get_return_by_id,
    commands::settings::fetch_settings_map,
    db::DbPool,
    errors::AppError,
    models::item::Item,
    models::print::{
        BarcodePrintData, InvoicePrintData, InvoicePrintItem, PurchasePrintData, PurchasePrintItem,
        ReturnPrintData, ReturnPrintItem, ShopInfo,
    },
    services::invoice_pdf as invoice_pdf_service,
};
use crate::commands::{auth::SessionStore, guard};

/// Returns the persistent directory used to store managed logo files.
/// Creates the directory if it does not already exist.
pub fn logo_dir() -> Result<std::path::PathBuf, AppError> {
    let data_dir = dirs::data_dir().ok_or_else(|| {
        AppError::new(
            "APP_DATA_DIR_ERROR",
            "تعذر تحديد مجلد بيانات التطبيق",
            "Unable to resolve application data directory",
        )
    })?;
    let logos_dir = data_dir.join("pos").join("logos");
    std::fs::create_dir_all(&logos_dir).map_err(|e| {
        AppError::new(
            "LOGO_DIR_ERROR",
            "تعذر إنشاء مجلد الشعارات",
            &format!("Failed to create logos directory: {e}"),
        )
    })?;
    Ok(logos_dir)
}

#[tauri::command]
pub async fn generate_invoice_pdf(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
    invoice_id: i64,
) -> Result<String, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let invoice = fetch_invoice_detail(&pool, invoice_id).await?;
    let mut settings = fetch_settings_map(&pool).await?;
    inject_customer_balance(&pool, &invoice, &mut settings).await?;
    let path = invoice_pdf_service::generate_invoice_pdf(&invoice, &settings)?;

    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn open_whatsapp_with_invoice(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
    invoice_id: i64,
    invoice_number: String,
) -> Result<bool, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let invoice = fetch_invoice_detail(&pool, invoice_id).await?;
    let mut settings = fetch_settings_map(&pool).await?;
    inject_customer_balance(&pool, &invoice, &mut settings).await?;
    let url = build_whatsapp_url(&settings, &invoice, &invoice_number);

    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|error| {
        AppError::new(
            "OPEN_URL_FAILED",
            "تعذر فتح واتساب",
            &format!("Failed to open WhatsApp URL: {error}"),
        )
    })?;

    Ok(true)
}

#[tauri::command]
pub async fn get_label_printer_list(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
) -> Result<Vec<String>, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    list_printers_impl()
}

#[tauri::command]
pub async fn list_printers(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
) -> Result<Vec<String>, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    list_printers_impl()
}

fn list_printers_impl() -> Result<Vec<String>, AppError> {
    match list_os_printers() {
        Ok(printers) => Ok(printers),
        Err(error) => {
            eprintln!("printer discovery failed: {}", error.message_en);
            Ok(Vec::new())
        }
    }
}

async fn shop_info_from_settings(pool: &DbPool) -> Result<ShopInfo, AppError> {
    let settings = fetch_settings_map(pool).await?;
    Ok(ShopInfo {
        shop_name: settings
            .get("shop_name")
            .cloned()
            .unwrap_or_else(|| "صفقة".to_owned()),
        shop_address: settings
            .get("shop_address")
            .cloned()
            .unwrap_or_else(|| "العنوان غير محدد".to_owned()),
        shop_phone: settings.get("shop_phone").cloned().unwrap_or_default(),
        shop_logo_path: settings
            .get("shop_logo_path")
            .cloned()
            .unwrap_or_default(),
    })
}

/// Reads the shop logo image at the given absolute path and returns it as a
/// base64-encoded data URL (e.g. `data:image/png;base64,...`).
/// Returns an empty string when the path is empty, missing, or unreadable.
#[tauri::command]
pub async fn read_logo_base64(
    sessions: State<'_, SessionStore>,
    pool: State<'_, DbPool>,
    token: Option<String>,
    logo_path: String,
) -> Result<String, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let trimmed = logo_path.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let path = std::path::Path::new(trimmed);
    if !path.exists() {
        return Ok(String::new());
    }

    let bytes = std::fs::read(path).unwrap_or_default();
    if bytes.is_empty() {
        return Ok(String::new());
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mime = match ext.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        _ => "image/png",
    };

    let encoded = logo_base64_encode(&bytes);
    Ok(format!("data:{};base64,{}", mime, encoded))
}

fn logo_base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 { chunk[1] as usize } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as usize } else { 0 };

        result.push(CHARS[b0 >> 2] as char);
        result.push(CHARS[((b0 & 3) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((b1 & 0xf) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

/// Copies the selected logo image into the persistent application-data logos
/// directory (`<data_dir>/pos/logos/shop_logo.<ext>`), validates the file type
/// and size (max 5 MB), then returns the new managed path.
///
/// The caller is responsible for persisting the returned path to settings.
#[tauri::command]
pub async fn save_logo(
    sessions: State<'_, SessionStore>,
    pool: State<'_, DbPool>,
    token: Option<String>,
    source_path: String,
) -> Result<String, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let trimmed = source_path.trim();
    if trimmed.is_empty() {
        return Err(AppError::new(
            "LOGO_EMPTY_PATH",
            "لم يتم تحديد ملف الشعار",
            "Logo source path is empty",
        ));
    }

    let src = std::path::Path::new(trimmed);
    if !src.exists() {
        return Err(AppError::new(
            "LOGO_NOT_FOUND",
            "ملف الشعار غير موجود",
            "Logo source file does not exist",
        ));
    }

    // Validate extension
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" => {}
        _ => {
            return Err(AppError::new(
                "LOGO_INVALID_TYPE",
                "نوع الملف غير مدعوم. الأنواع المدعومة: PNG، JPEG، WEBP",
                &format!("Unsupported logo file type: .{ext}"),
            ));
        }
    }

    // Validate file size (5 MB limit)
    let metadata = std::fs::metadata(src).map_err(|e| {
        AppError::new(
            "LOGO_STAT_ERROR",
            "تعذر قراءة معلومات ملف الشعار",
            &format!("Failed to stat logo file: {e}"),
        )
    })?;
    const MAX_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
    if metadata.len() > MAX_SIZE {
        return Err(AppError::new(
            "LOGO_TOO_LARGE",
            "حجم الشعار يتجاوز الحد الأقصى المسموح به (5 ميجابايت)",
            &format!("Logo file size {} bytes exceeds limit {MAX_SIZE}", metadata.len()),
        ));
    }

    // Determine managed destination
    let logos_dir = logo_dir()?;
    let dest_name = format!("shop_logo.{ext}");
    let dest = logos_dir.join(&dest_name);

    // If source is already the managed destination, skip the copy
    let src_canonical = std::fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    let dest_canonical = std::fs::canonicalize(&dest).unwrap_or_else(|_| dest.clone());
    if src_canonical == dest_canonical {
        return Ok(dest.to_string_lossy().into_owned());
    }

    // Remove any pre-existing managed logo with a different extension
    for other_ext in ["png", "jpg", "jpeg", "webp", "gif", "bmp"] {
        if other_ext != ext.as_str() {
            let other = logos_dir.join(format!("shop_logo.{other_ext}"));
            if other.exists() {
                let _ = std::fs::remove_file(&other);
            }
        }
    }

    std::fs::copy(src, &dest).map_err(|e| {
        AppError::new(
            "LOGO_COPY_ERROR",
            "تعذر نسخ ملف الشعار",
            &format!("Failed to copy logo: {e}"),
        )
    })?;

    Ok(dest.to_string_lossy().into_owned())
}

/// Deletes the managed logo file and returns `true`.
/// Returns `true` even if no logo was stored (idempotent).
#[tauri::command]
pub async fn delete_logo(
    sessions: State<'_, SessionStore>,
    pool: State<'_, DbPool>,
    token: Option<String>,
) -> Result<bool, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let logos_dir = match logo_dir() {
        Ok(d) => d,
        Err(_) => return Ok(true),
    };

    for ext in ["png", "jpg", "jpeg", "webp", "gif", "bmp"] {
        let candidate = logos_dir.join(format!("shop_logo.{ext}"));
        if candidate.exists() {
            let _ = std::fs::remove_file(&candidate);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod logo_tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "safqah-logo-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir should be writable");
        dir
    }

    #[test]
    fn base64_encode_roundtrip() {
        // "Man" → "TWFu" per RFC 4648
        assert_eq!(logo_base64_encode(b"Man"), "TWFu");
        // Padding: "M" → "TQ=="
        assert_eq!(logo_base64_encode(b"M"), "TQ==");
        // Padding: "Ma" → "TWE="
        assert_eq!(logo_base64_encode(b"Ma"), "TWE=");
        // Empty input
        assert_eq!(logo_base64_encode(b""), "");
    }

    #[test]
    fn logo_base64_encode_produces_only_valid_chars() {
        let payload: Vec<u8> = (0u8..=255u8).collect();
        let encoded = logo_base64_encode(&payload);
        let valid: std::collections::HashSet<char> =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/="
                .chars()
                .collect();
        for ch in encoded.chars() {
            assert!(valid.contains(&ch), "unexpected char: {ch}");
        }
    }

    #[test]
    fn read_logo_returns_empty_for_missing_file() {
        let missing = "/nonexistent/path/logo.png";
        let path = std::path::Path::new(missing);
        // Replicating the logic from read_logo_base64 synchronously
        let result = if !path.exists() {
            String::new()
        } else {
            "should not reach".to_owned()
        };
        assert_eq!(result, "");
    }

    #[test]
    fn save_logo_logic_copies_file_to_managed_dir() {
        let tmp = temp_dir();
        // Create a fake PNG source file
        let src = tmp.join("my_logo.png");
        std::fs::write(&src, b"\x89PNG\r\n\x1a\n fake png bytes").unwrap();

        // Simulate the destination logic
        let logos_dir = tmp.join("logos");
        std::fs::create_dir_all(&logos_dir).unwrap();
        let dest = logos_dir.join("shop_logo.png");
        std::fs::copy(&src, &dest).unwrap();

        assert!(dest.exists(), "managed logo should exist");
        assert_eq!(
            std::fs::read(&dest).unwrap(),
            std::fs::read(&src).unwrap(),
            "content must match"
        );

        // Moving/deleting the original should not affect the managed copy
        std::fs::remove_file(&src).unwrap();
        assert!(dest.exists(), "managed copy survives original deletion");
    }

    #[test]
    fn save_logo_logic_cleans_up_old_extension() {
        let tmp = temp_dir();
        let logos_dir = tmp.join("logos");
        std::fs::create_dir_all(&logos_dir).unwrap();

        // Existing PNG logo
        let old_png = logos_dir.join("shop_logo.png");
        std::fs::write(&old_png, b"old png").unwrap();

        // User selects a JPEG
        let src_jpeg = tmp.join("new_logo.jpeg");
        std::fs::write(&src_jpeg, b"new jpeg bytes").unwrap();

        // Simulate cleanup + copy
        for ext in ["png", "jpg", "jpeg", "webp", "gif", "bmp"] {
            if ext != "jpeg" {
                let other = logos_dir.join(format!("shop_logo.{ext}"));
                if other.exists() {
                    let _ = std::fs::remove_file(&other);
                }
            }
        }
        let dest_jpeg = logos_dir.join("shop_logo.jpeg");
        std::fs::copy(&src_jpeg, &dest_jpeg).unwrap();

        assert!(!old_png.exists(), "old PNG should be removed");
        assert!(dest_jpeg.exists(), "new JPEG should be present");
    }
}

async fn fetch_barcode_label_item(pool: &DbPool, item_id: i64) -> Result<Item, AppError> {
    sqlx::query_as::<_, Item>(
        r#"
        SELECT *
        FROM items
        WHERE id = ? AND is_active = 1
        "#,
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::not_found("الصنف"))
}

async fn fetch_customer_balance(pool: &DbPool, customer_id: i64) -> Result<Option<i64>, AppError> {
    let balance: Option<(i64,)> =
        sqlx::query_as("SELECT balance_millieme FROM customers WHERE id = ?")
            .bind(customer_id)
            .fetch_optional(pool)
            .await?;
    Ok(balance.map(|(b,)| b))
}

#[tauri::command]
pub async fn get_invoice_print_data(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
    invoice_id: i64,
) -> Result<InvoicePrintData, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let invoice = fetch_invoice_detail(&pool, invoice_id).await?;
    let shop = shop_info_from_settings(&pool).await?;

    let customer_balance_millieme = match invoice.customer_id {
        Some(customer_id) => fetch_customer_balance(&pool, customer_id).await?,
        None => None,
    };

    Ok(InvoicePrintData {
        invoice_number: invoice.invoice_number,
        customer_name: invoice.customer_name,
        cashier_name: invoice.cashier_name,
        subtotal_millieme: invoice.subtotal_millieme,
        discount_millieme: invoice.discount_millieme,
        tax_millieme: invoice.tax_millieme,
        total_millieme: invoice.total_millieme,
        paid_millieme: invoice.paid_millieme,
        payment_method: invoice.payment_method,
        status: invoice.status,
        notes: invoice.notes,
        created_at: invoice.created_at,
        items: invoice
            .items
            .into_iter()
            .map(|item| InvoicePrintItem {
                item_name_ar: item.item_name_ar,
                qty: item.qty,
                unit_price_millieme: item.unit_price_millieme,
                discount_millieme: item.discount_millieme,
                total_millieme: item.total_millieme,
            })
            .collect(),
        shop,
        customer_balance_millieme,
    })
}

#[tauri::command]
pub async fn get_purchase_print_data(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
    purchase_id: i64,
) -> Result<PurchasePrintData, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let purchase = get_purchase_detail_impl(&pool, purchase_id).await?;
    let shop = shop_info_from_settings(&pool).await?;

    Ok(PurchasePrintData {
        invoice_number: purchase.invoice_number,
        supplier_name: purchase.supplier_name,
        subtotal_millieme: purchase.subtotal_millieme,
        discount_millieme: purchase.discount_millieme,
        total_millieme: purchase.total_millieme,
        paid_millieme: purchase.paid_millieme,
        payment_method: purchase.payment_method,
        status: purchase.status,
        notes: purchase.notes,
        created_at: purchase.created_at,
        items: purchase
            .items
            .into_iter()
            .map(|item| PurchasePrintItem {
                item_name_ar: item.item_name_ar,
                qty: item.qty,
                unit_cost_millieme: item.unit_cost_millieme,
                total_millieme: item.total_millieme,
            })
            .collect(),
        shop,
    })
}

#[tauri::command]
pub async fn get_return_print_data(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
    return_id: i64,
) -> Result<ReturnPrintData, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let return_data = get_return_by_id(&pool, return_id).await?;

    let original_invoice_number: String =
        sqlx::query_scalar("SELECT invoice_number FROM invoices WHERE id = ?")
            .bind(return_data.original_invoice_id)
            .fetch_optional(&*pool)
            .await?
            .ok_or_else(|| AppError::not_found("الفاتورة"))?;

    let item_names: HashMap<i64, String> = sqlx::query_as(
        "SELECT DISTINCT ri.item_id, i.name_ar FROM return_items ri JOIN items i ON i.id = ri.item_id WHERE ri.return_id = ?",
    )
    .bind(return_id)
    .fetch_all(&*pool)
    .await?
    .into_iter()
    .collect();

    let shop = shop_info_from_settings(&pool).await?;

    Ok(ReturnPrintData {
        return_number: return_data.return_number,
        original_invoice_id: return_data.original_invoice_id,
        original_invoice_number,
        total_millieme: return_data.total_millieme,
        refund_method: return_data.refund_method,
        status: return_data.status,
        notes: return_data.notes,
        created_at: return_data.created_at,
        items: return_data
            .items
            .into_iter()
            .map(|item| ReturnPrintItem {
                item_name_ar: item_names
                    .get(&item.item_id)
                    .cloned()
                    .unwrap_or_default(),
                qty: item.qty,
                unit_price_millieme: item.unit_price_millieme,
                total_millieme: item.total_millieme,
            })
            .collect(),
        shop,
    })
}

#[tauri::command]
pub async fn get_barcode_print_data(
    pool: State<'_, DbPool>,
    sessions: State<'_, SessionStore>,
    token: Option<String>,
    item_ids: Vec<i64>,
) -> Result<Vec<BarcodePrintData>, AppError> {
    guard::require_role(&sessions, &pool, token, guard::ANY_ROLE).await?;

    let mut results = Vec::with_capacity(item_ids.len());

    for item_id in item_ids {
        let item = fetch_barcode_label_item(&pool, item_id).await?;

        let barcode = item
            .barcode
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::new(
                    "NO_BARCODE",
                    &format!("الصنف {} لا يحتوي على باركود", item.name_ar),
                    "Item has no barcode",
                )
            })?;

        results.push(BarcodePrintData {
            item_id,
            barcode: barcode.to_owned(),
            name_ar: item.name_ar,
            sell_price_millieme: item.sell_price_millieme,
        });
    }

    Ok(results)
}

async fn inject_customer_balance(
    pool: &DbPool,
    invoice: &crate::models::sale::InvoiceDetail,
    settings: &mut HashMap<String, String>,
) -> Result<(), AppError> {
    let Some(customer_id) = invoice.customer_id else {
        return Ok(());
    };

    let balance: Option<(i64,)> = sqlx::query_as("SELECT balance_millieme FROM customers WHERE id = ?")
        .bind(customer_id)
        .fetch_optional(pool)
        .await?;

    if let Some((balance_millieme,)) = balance {
        settings.insert(
            format!("customer_balance_{customer_id}"),
            balance_millieme.to_string(),
        );
    }

    Ok(())
}

fn build_whatsapp_url(
    settings: &HashMap<String, String>,
    invoice: &crate::models::sale::InvoiceDetail,
    invoice_number: &str,
) -> String {
    let shop_name = setting_text(settings, "shop_name", "صفقة");
    let thank_you_message = setting_text(
        settings,
        "thank_you_message",
        "شكراً لزيارتكم — نتطلع لخدمتكم مجدداً",
    );

    let items_list = invoice
        .items
        .iter()
        .map(|item| {
            format!(
                "• {} × {} = {} جنيه",
                item.item_name_ar,
                item.qty,
                format_money_plain(item.total_millieme)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let payment_method = payment_method_arabic(&invoice.payment_method);
    let status = status_arabic(&invoice.status);
    let balance_line = customer_balance_line(settings, invoice);

    let mut message = format!(
        "🧾 فاتورة من {shop_name}\n\
         رقم الفاتورة: {invoice_number}\n\
         ──────────────\n\
         {items_list}\n\
         ──────────────\n\
         الإجمالي: {total} جنيه\n\
         المدفوع: {paid} جنيه\n\
         طريقة الدفع: {method}\n\
         الحالة: {status}",
        total = format_money_plain(invoice.total_millieme),
        paid = format_money_plain(invoice.paid_millieme),
        method = payment_method,
        status = status,
    );

    if !balance_line.is_empty() {
        message.push('\n');
        message.push_str(&balance_line);
    }

    message.push_str("\n──────────────\n");
    message.push_str(&thank_you_message);

    format!("https://wa.me/?text={}", urlencoding::encode(&message))
}

fn customer_balance_line(
    settings: &HashMap<String, String>,
    invoice: &crate::models::sale::InvoiceDetail,
) -> String {
    let Some(customer_id) = invoice.customer_id else {
        return String::new();
    };

    let Some(balance_value) = settings.get(&format!("customer_balance_{customer_id}")) else {
        return String::new();
    };

    let Ok(balance_millieme) = balance_value.trim().parse::<i64>() else {
        return String::new();
    };

    if balance_millieme > 0 {
        format!(
            "💳 رصيد مستحق: {} جنيه",
            format_money_plain(balance_millieme)
        )
    } else if balance_millieme < 0 {
        format!(
            "💚 رصيد لصالحك: {} جنيه",
            format_money_plain(balance_millieme.abs())
        )
    } else {
        String::new()
    }
}

fn setting_text(settings: &HashMap<String, String>, key: &str, default: &str) -> String {
    settings
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or(default)
        .to_owned()
}

fn payment_method_arabic(method: &str) -> &'static str {
    match method {
        "cash" => "نقدي",
        "card" => "فيزا/كارت",
        "deferred" => "آجل",
        "split" => "دفع مختلط",
        _ => "غير محددة",
    }
}

fn status_arabic(status: &str) -> &'static str {
    match status {
        "paid" => "مدفوع ✓",
        "deferred" => "آجل",
        "partial" => "مدفوع جزئياً",
        _ => "غير محددة",
    }
}

fn format_money_plain(millieme: i64) -> String {
    format!("{:.2}", millieme as f64 / 1000.0)
}


fn list_os_printers() -> Result<Vec<String>, AppError> {
    #[cfg(target_os = "windows")]
    {
        let output = crate::services::windows_process::no_window_command("wmic")
            .args(["printer", "get", "name"])
            .output()
            .map_err(list_printers_error)?;

        if !output.status.success() {
            return Err(list_printers_error(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr).to_string(),
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && *line != "Name")
            .map(ToOwned::to_owned)
            .collect())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("lpstat")
            .arg("-a")
            .output()
            .map_err(list_printers_error)?;

        if !output.status.success() {
            return Err(list_printers_error(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr).to_string(),
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(ToOwned::to_owned)
            .collect())
    }
}

fn list_printers_error(error: std::io::Error) -> AppError {
    AppError::new(
        "LIST_PRINTERS_FAILED",
        "تعذر قراءة قائمة الطابعات",
        &format!("Failed to list printers: {error}"),
    )
}
