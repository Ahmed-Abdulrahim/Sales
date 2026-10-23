/// Generates the printable PDF for a sales invoice.
///
/// Layout mirrors the HTML invoice template in `invoicePrint.ts`:
///   - Shop name (upper-right), logo placeholder (upper-left)
///   - Prominent invoice-number bar
///   - Customer / seller row
///   - Date / time row (separately formatted from the saved timestamp)
///   - RTL item table: م | اسم الصنف | الكمية | سعر الوحدة | الإجمالي
///   - Totals: subtotal, conditional discount, net total (صافي الفاتورة)
///   - Footer: شكرًا لثقتكم في معرض أولاد عبد الرحيم للأدوات الصحية
///
/// No payment-method, tax, installment, debt, product-code or status fields.
use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Cursor},
    path::PathBuf,
};

use arabic_reshaper::arabic_reshape;
use printpdf::{Line, Mm, PdfDocument, Point};
use unicode_bidi::BidiInfo;

use crate::{errors::AppError, models::sale::InvoiceDetail};

// A4 page, 14 mm margins on all sides
const PAGE_WIDTH_MM: f64 = 210.0;
const PAGE_HEIGHT_MM: f64 = 297.0;
const MARGIN_MM: f64 = 14.0;
const TEXT_WIDTH_MM: f64 = PAGE_WIDTH_MM - MARGIN_MM * 2.0; // 182 mm

const FOOTER_MSG: &str = "شكرًا لثقتكم في معرض أولاد عبد الرحيم للأدوات الصحية";

pub fn generate_invoice_pdf(
    invoice: &InvoiceDetail,
    settings: &HashMap<String, String>,
) -> Result<PathBuf, AppError> {
    let (doc, page1, layer1) = PdfDocument::new(
        &format!("فاتورة {}", invoice.invoice_number),
        Mm(PAGE_WIDTH_MM as f32),
        Mm(PAGE_HEIGHT_MM as f32),
        "Layer 1",
    );
    let layer = doc.get_page(page1).get_layer(layer1);
    let font = doc
        .add_external_font(Cursor::new(include_bytes!(
            "../../resources/fonts/Amiri-Regular.ttf"
        )))
        .map_err(pdf_error)?;

    // ── settings ──────────────────────────────────────────────────────────
    let shop_name = setting_text(
        settings,
        "shop_name",
        "معرض أولاد عبد الرحيم للأدوات الصحية",
    );
    let shop_address = setting_text_opt(settings, "shop_address");
    let shop_phone = setting_text_opt(settings, "shop_phone");

    // ── timestamp parsing ──────────────────────────────────────────────────
    // created_at is stored as "YYYY-MM-DD HH:MM:SS" or ISO 8601
    let (date_str, time_str) = split_datetime(&invoice.created_at);
    let customer_str = invoice
        .customer_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("عميل عام");
    let cashier_str = invoice
        .cashier_name
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("");

    // ── layout cursor (y descends from top-margin) ────────────────────────
    let top_y = PAGE_HEIGHT_MM - MARGIN_MM;
    let right_x = PAGE_WIDTH_MM - MARGIN_MM; // text anchored here for RTL
    let left_x = MARGIN_MM;
    let mut y = top_y;

    // ═══════════════════════════════════════════════════════════════════════
    // HEADER — shop name right-aligned, separator below
    // ═══════════════════════════════════════════════════════════════════════
    draw_text_right(&layer, &font, &shop_name, right_x, y, 14.0);
    y -= 9.0;

    if let Some(ref addr) = shop_address {
        if addr != "العنوان" && addr != "العنوان غير محدد" {
            draw_text_right(&layer, &font, addr, right_x, y, 9.0);
            y -= 6.0;
        }
    }
    if let Some(ref phone) = shop_phone {
        draw_text_right(&layer, &font, phone, right_x, y, 9.0);
        y -= 6.0;
    }

    y -= 2.0;
    draw_h_line(&layer, left_x, right_x, y, 0.7);
    y -= 8.0;

    // ═══════════════════════════════════════════════════════════════════════
    // INVOICE-ID BAR
    // ═══════════════════════════════════════════════════════════════════════
    // Label on right, number on left (RTL: the number ends up prominent)
    draw_text_right(
        &layer,
        &font,
        &format!("رقم الفاتورة: {}", invoice.invoice_number),
        right_x,
        y,
        13.0,
    );
    y -= 9.0;
    draw_h_line(&layer, left_x, right_x, y, 0.4);
    y -= 8.0;

    // ═══════════════════════════════════════════════════════════════════════
    // METADATA — two rows of two columns each
    // ═══════════════════════════════════════════════════════════════════════
    // Row 1: customer (right half) | cashier/seller (left half)
    let mid_x = left_x + TEXT_WIDTH_MM / 2.0;
    draw_text_right(
        &layer,
        &font,
        &format!("العميل: {customer_str}"),
        right_x,
        y,
        10.5,
    );
    if !cashier_str.is_empty() {
        draw_text_right(
            &layer,
            &font,
            &format!("البائع: {cashier_str}"),
            mid_x,
            y,
            10.5,
        );
    }
    y -= 7.0;

    // Row 2: date (right half) | time (left half)
    draw_text_right(
        &layer,
        &font,
        &format!("التاريخ: {date_str}"),
        right_x,
        y,
        10.5,
    );
    draw_text_right(&layer, &font, &format!("الوقت: {time_str}"), mid_x, y, 10.5);
    y -= 7.0;

    y -= 2.0;
    draw_h_line(&layer, left_x, right_x, y, 0.6);
    y -= 9.0;

    // ═══════════════════════════════════════════════════════════════════════
    // ITEMS TABLE
    // RTL column order (right→left): م | اسم الصنف | الكمية | سعر الوحدة | الإجمالي
    // Column widths in mm (sum = TEXT_WIDTH_MM = 182)
    // ═══════════════════════════════════════════════════════════════════════
    //   م        :  10 mm
    //   اسم الصنف:  82 mm
    //   الكمية   :  20 mm
    //   سعر الوحدة:  35 mm
    //   الإجمالي :  35 mm
    let col_idx_w: f64 = 10.0;
    let col_name_w: f64 = 82.0;
    let col_qty_w: f64 = 20.0;
    let col_price_w: f64 = 35.0;
    let col_total_w: f64 = 35.0;

    // Column right-edges (RTL: first col is rightmost)
    let idx_right = right_x;
    let name_right = idx_right - col_idx_w;
    let qty_right = name_right - col_name_w;
    let price_right = qty_right - col_qty_w;
    let total_right = price_right - col_price_w;
    // total_right - col_total_w == left_x (check: 10+82+20+35+35=182 ✓)

    // Header row
    draw_text_right(&layer, &font, "م", idx_right, y, 10.0);
    draw_text_right(&layer, &font, "اسم الصنف", name_right, y, 10.0);
    draw_text_right(&layer, &font, "الكمية", qty_right, y, 10.0);
    draw_text_right(&layer, &font, "سعر الوحدة", price_right, y, 10.0);
    draw_text_right(&layer, &font, "الإجمالي", total_right, y, 10.0);
    y -= 2.0;
    draw_h_line(&layer, left_x, right_x, y, 0.4);
    y -= 7.0;

    // Item rows
    for (i, item) in invoice.items.iter().enumerate() {
        draw_text_right(&layer, &font, &(i + 1).to_string(), idx_right, y, 9.5);
        // Name may be long — truncate rather than overflow column
        let name = truncate_for_column(&item.item_name_ar, col_name_w, 9.5);
        draw_text_right(&layer, &font, &name, name_right, y, 9.5);
        draw_text_right(&layer, &font, &item.qty.to_string(), qty_right, y, 9.5);
        draw_text_right(
            &layer,
            &font,
            &format_money_plain(item.unit_price_millieme),
            price_right,
            y,
            9.5,
        );
        draw_text_right(
            &layer,
            &font,
            &format_money_plain(item.total_millieme),
            total_right,
            y,
            9.5,
        );
        y -= 2.0;
        draw_h_line(&layer, left_x, right_x, y, 0.15);
        y -= 6.0;
    }

    y -= 2.0;
    draw_h_line(&layer, left_x, right_x, y, 0.5);
    y -= 10.0;

    // ═══════════════════════════════════════════════════════════════════════
    // TOTALS  (right-aligned label + value, in a compact block)
    // ═══════════════════════════════════════════════════════════════════════
    let totals_right = right_x;
    let totals_value_right = totals_right - 5.0; // value anchored 5 mm from page right
    let totals_label_right = totals_value_right - 42.0; // label just left of the value

    draw_text_right(
        &layer,
        &font,
        "الإجمالي قبل الخصم",
        totals_label_right,
        y,
        10.5,
    );
    draw_text_right(
        &layer,
        &font,
        &format_money_plain(invoice.subtotal_millieme),
        totals_value_right,
        y,
        10.5,
    );
    y -= 7.0;

    if invoice.discount_millieme > 0 {
        draw_text_right(&layer, &font, "الخصم", totals_label_right, y, 10.5);
        draw_text_right(
            &layer,
            &font,
            &format_money_plain(invoice.discount_millieme),
            totals_value_right,
            y,
            10.5,
        );
        y -= 7.0;
    }

    // Net total — bold / larger
    draw_h_line(&layer, left_x, right_x, y + 1.0, 0.5);
    y -= 3.0;
    draw_text_right(&layer, &font, "صافي الفاتورة", totals_label_right, y, 12.5);
    draw_text_right(
        &layer,
        &font,
        &format_money_plain(invoice.total_millieme),
        totals_value_right,
        y,
        12.5,
    );
    y -= 3.0;
    draw_h_line(&layer, left_x, right_x, y, 0.5);
    y -= 14.0;

    // ═══════════════════════════════════════════════════════════════════════
    // FOOTER
    // ═══════════════════════════════════════════════════════════════════════
    draw_h_line(&layer, left_x, right_x, y + 3.0, 0.7);
    draw_centered_text(&layer, &font, FOOTER_MSG, 10.5, y);

    // ── save PDF ──────────────────────────────────────────────────────────
    // Save to the persistent app-data directory so the file survives until
    // the user dismisses the WhatsApp dialog.
    let out_dir = pdf_output_dir()?;
    let out_path = out_dir.join(format!("invoice_{}.pdf", invoice.id));
    let file = File::create(&out_path).map_err(pdf_error)?;
    doc.save(&mut BufWriter::new(file)).map_err(pdf_error)?;

    Ok(out_path)
}

/// Returns (or creates) the directory used for generated invoice PDFs.
/// Uses `<data_dir>/pos/invoices/` so the files are persistent across updates.
fn pdf_output_dir() -> Result<PathBuf, AppError> {
    let data_dir = dirs::data_dir().ok_or_else(|| {
        AppError::new(
            "APP_DATA_DIR_ERROR",
            "تعذر تحديد مجلد بيانات التطبيق",
            "Unable to resolve application data directory",
        )
    })?;
    let out_dir = data_dir.join("pos").join("invoices");
    std::fs::create_dir_all(&out_dir).map_err(|e| {
        AppError::new(
            "PDF_DIR_ERROR",
            "تعذر إنشاء مجلد الفواتير",
            &format!("Failed to create invoices directory: {e}"),
        )
    })?;
    Ok(out_dir)
}

/// Splits a stored timestamp (either "YYYY-MM-DD HH:MM:SS" or ISO 8601) into
/// a (date_str, time_str) pair.  Keeps both strings in simple ASCII/Arabic
/// numerals since Amiri handles these correctly.
fn split_datetime(created_at: &str) -> (String, String) {
    // Try splitting on a space or 'T'
    let sep = if created_at.contains('T') { 'T' } else { ' ' };
    let mut parts = created_at.splitn(2, sep);
    let date = parts.next().unwrap_or(created_at).to_owned();
    let time = parts
        .next()
        .map(|t| {
            // Drop any timezone suffix: take only "HH:MM" or "HH:MM:SS"
            t.split('+').next().unwrap_or(t).trim().to_owned()
        })
        .unwrap_or_default();
    (date, time)
}

/// Estimates how many characters of `text` fit in `column_width_mm` at
/// `font_size_pt` using the same rough formula as the rest of the drawing code.
fn truncate_for_column(text: &str, column_width_mm: f64, font_size_pt: f64) -> String {
    let shaped = shape_rtl(text);
    let max_chars = (column_width_mm / (font_size_pt * 0.32)).floor() as usize;
    if shaped.chars().count() <= max_chars {
        return shaped;
    }
    let mut truncated: String = shaped.chars().take(max_chars.saturating_sub(1)).collect();
    truncated.push('…');
    truncated
}

// ─── drawing helpers ─────────────────────────────────────────────────────────

fn draw_centered_text(
    layer: &printpdf::PdfLayerReference,
    font: &printpdf::IndirectFontRef,
    text: &str,
    font_size: f64,
    y: f64,
) {
    let shaped = shape_rtl(text);
    let width = estimate_text_width_mm(&shaped, font_size);
    let x = ((PAGE_WIDTH_MM - width) / 2.0).max(MARGIN_MM);
    layer.use_text(shaped, font_size as f32, Mm(x as f32), Mm(y as f32), font);
}

fn draw_text_right(
    layer: &printpdf::PdfLayerReference,
    font: &printpdf::IndirectFontRef,
    text: &str,
    right_edge: f64,
    y: f64,
    font_size: f64,
) {
    let shaped = shape_rtl(text);
    let width = estimate_text_width_mm(&shaped, font_size);
    let x = (right_edge - width).max(MARGIN_MM);
    layer.use_text(shaped, font_size as f32, Mm(x as f32), Mm(y as f32), font);
}

fn draw_h_line(
    layer: &printpdf::PdfLayerReference,
    x1: f64,
    x2: f64,
    y: f64,
    thickness: f64,
) {
    layer.set_outline_thickness(thickness as f32);
    let line = Line {
        points: vec![
            (Point::new(Mm(x1 as f32), Mm(y as f32)), false),
            (Point::new(Mm(x2 as f32), Mm(y as f32)), false),
        ],
        is_closed: false,
    };
    layer.add_line(line);
}

fn shape_rtl(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let reshaped = arabic_reshape(text);
    let bidi = BidiInfo::new(&reshaped, None);
    if bidi.paragraphs.is_empty() {
        return reshaped;
    }
    let paragraph = &bidi.paragraphs[0];
    bidi.reorder_line(paragraph, 0..reshaped.len()).to_string()
}

fn estimate_text_width_mm(text: &str, font_size: f64) -> f64 {
    text.chars().count() as f64 * font_size * 0.32
}

fn format_money_plain(millieme: i64) -> String {
    format!("{:.2}", millieme as f64 / 1000.0)
}

fn setting_text(settings: &HashMap<String, String>, key: &str, default: &str) -> String {
    settings
        .get(key)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .unwrap_or(default)
        .to_owned()
}

fn setting_text_opt(settings: &HashMap<String, String>, key: &str) -> Option<String> {
    settings
        .get(key)
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

fn pdf_error(error: impl std::fmt::Display) -> AppError {
    AppError::new(
        "PDF_ERROR",
        "تعذر إنشاء ملف PDF",
        &format!("PDF generation failed: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_datetime_handles_space_separator() {
        let (d, t) = split_datetime("2026-10-23 18:05:03");
        assert_eq!(d, "2026-10-23");
        assert_eq!(t, "18:05:03");
    }

    #[test]
    fn split_datetime_handles_t_separator() {
        let (d, t) = split_datetime("2026-10-23T18:05:03+03:00");
        assert_eq!(d, "2026-10-23");
        assert_eq!(t, "18:05:03");
    }

    #[test]
    fn split_datetime_handles_date_only() {
        let (d, t) = split_datetime("2026-10-23");
        assert_eq!(d, "2026-10-23");
        assert_eq!(t, "");
    }

    #[test]
    fn format_money_plain_two_decimals() {
        assert_eq!(format_money_plain(150_000), "150.00");
        assert_eq!(format_money_plain(150_500), "150.50");
        assert_eq!(format_money_plain(0), "0.00");
    }

    #[test]
    fn truncate_for_column_keeps_short_text() {
        // 10 chars × 9.5 pt × 0.32 ≈ 30.4 mm — fits in 50 mm
        let text = "أدوات صحية";
        let result = truncate_for_column(text, 50.0, 9.5);
        assert_eq!(result.chars().filter(|&c| c != '…').count(), text.chars().count());
    }

    #[test]
    fn truncate_for_column_truncates_long_text() {
        // 100-char string won't fit in 30 mm at 9.5 pt
        let text = "أ".repeat(100);
        let result = truncate_for_column(&text, 30.0, 9.5);
        assert!(result.ends_with('…'), "should end with ellipsis");
        let visible: String = result.chars().filter(|&c| c != '…').collect();
        let max_chars = (30.0_f64 / (9.5 * 0.32)).floor() as usize;
        assert!(visible.chars().count() < max_chars + 1);
    }

    #[test]
    fn setting_text_returns_default_for_missing_key() {
        let map = HashMap::new();
        assert_eq!(setting_text(&map, "shop_name", "fallback"), "fallback");
    }

    #[test]
    fn setting_text_returns_default_for_whitespace_value() {
        let mut map = HashMap::new();
        map.insert("shop_name".to_owned(), "   ".to_owned());
        assert_eq!(setting_text(&map, "shop_name", "fallback"), "fallback");
    }

    #[test]
    fn setting_text_opt_returns_none_for_empty() {
        let mut map = HashMap::new();
        map.insert("shop_address".to_owned(), "".to_owned());
        assert!(setting_text_opt(&map, "shop_address").is_none());
    }
}
