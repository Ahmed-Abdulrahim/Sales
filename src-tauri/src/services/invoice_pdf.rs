/// Generates a printable HTML invoice file for a sales invoice.
///
/// Instead of producing a binary PDF (which requires unavailable crates in the
/// current build environment), this service writes a self-contained, print-ready
/// HTML file to the persistent app-data directory.  The file is opened by the
/// frontend in the default browser; the user can then use Ctrl+P / Cmd+P to
/// print or save as PDF.  The HTML layout mirrors `invoicePrint.ts` exactly:
///
///   - Traditional Arabic A4 layout with RTL text direction
///   - Shop name / logo header
///   - Customer info (right) | cashier / date (left)
///   - 7-column item table: م | اسم الصنف | كمية | سعر | خصم% | صافي السعر | قيمة
///   - Totals: الإجمالي, صافي الخصم, صافي الفاتورة, مدفوع نقدا, الرصيد الحالي
///   - Footer: amount in Arabic words + شكرًا لثقتكم في معرض أولاد عبد الرحيم للأدوات الصحية
use std::{collections::HashMap, path::PathBuf};

use crate::{errors::AppError, models::sale::InvoiceDetail};

const SHOP_NAME_DEFAULT: &str = "معرض أولاد عبد الرحيم للأدوات الصحية";

pub fn generate_invoice_pdf(
    invoice: &InvoiceDetail,
    settings: &HashMap<String, String>,
) -> Result<PathBuf, AppError> {
    // ── settings ────────────────────────────────────────────────────────────
    let shop_name = setting_text(settings, "shop_name", SHOP_NAME_DEFAULT);
    let shop_address = setting_text_opt(settings, "shop_address");
    let shop_phone = setting_text_opt(settings, "shop_phone");
    let logo_path = settings
        .get("shop_logo_path")
        .map(|p| p.trim().to_owned())
        .filter(|p| !p.is_empty());

    // ── logo base64 (embed inline so the HTML file is self-contained) ────────
    let logo_html = match &logo_path {
        Some(path) => embed_logo(path),
        None => String::new(),
    };

    // ── timestamp ──────────────────────────────────────────────────────────
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
        .unwrap_or("—");

    // ── per-item rows ───────────────────────────────────────────────────────
    let item_rows: String = invoice
        .items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let gross = item.qty * item.unit_price_millieme;
            let net_line = item.total_millieme;
            let disc_total = gross - net_line;
            let disc_pct = if gross > 0 {
                (disc_total * 100 / gross)
            } else {
                0
            };
            let net_unit = if item.qty > 0 {
                net_line / item.qty
            } else {
                item.unit_price_millieme
            };
            let row_class = if i % 2 == 0 { "row-e" } else { "row-o" };
            format!(
                "<tr class=\"{row_class}\">
                  <td class=\"c-num\">{num}</td>
                  <td class=\"c-name\">{name}</td>
                  <td class=\"c-qty\">{qty}</td>
                  <td class=\"c-price\">{price}</td>
                  <td class=\"c-disc\">{disc}</td>
                  <td class=\"c-net\">{net}</td>
                  <td class=\"c-total\">{total}</td>
                </tr>",
                num = i + 1,
                name = esc(&item.item_name_ar),
                qty = item.qty,
                price = fmt_money(item.unit_price_millieme),
                disc = disc_pct,
                net = fmt_money(net_unit),
                total = fmt_money(net_line),
            )
        })
        .collect();

    // ── totals ─────────────────────────────────────────────────────────────
    let subtotal = invoice.subtotal_millieme;
    let discount = invoice.discount_millieme;
    let net_total = invoice.total_millieme;
    let paid = invoice.paid_millieme;

    // Customer balance (injected into settings map by the caller)
    let customer_balance: Option<i64> = invoice
        .customer_id
        .and_then(|cid| settings.get(&format!("customer_balance_{cid}")))
        .and_then(|v| v.trim().parse().ok());

    let balance_html = if let Some(bal) = customer_balance {
        let dir = if bal > 0 {
            "عليه"
        } else if bal < 0 {
            "له"
        } else {
            ""
        };
        let abs_bal = fmt_money(bal.abs());
        format!(
            "<div class=\"balance-section\">
               <span class=\"balance-label\">الرصيد الحالي :</span>
               <span class=\"balance-value\">{abs_bal}</span>
               {dir_span}
             </div>",
            dir_span = if dir.is_empty() {
                String::new()
            } else {
                format!("<span class=\"balance-dir\">{dir}</span>")
            }
        )
    } else {
        String::new()
    };

    // ── amount in words ────────────────────────────────────────────────────
    let words = millieme_to_arabic_words(net_total);

    // ── optional address/phone lines ───────────────────────────────────────
    let address_html = match &shop_address {
        Some(a)
            if !a.is_empty() && a != "العنوان" && a != "العنوان غير محدد" =>
        {
            format!("<div style=\"font-size:8.5pt;margin-top:2pt;\">{}</div>", esc(a))
        }
        _ => String::new(),
    };
    let phone_html = match &shop_phone {
        Some(p) if !p.is_empty() => {
            format!("<div style=\"font-size:8.5pt;\">{}</div>", esc(p))
        }
        _ => String::new(),
    };

    // ── discount row ───────────────────────────────────────────────────────
    let discount_row = format!(
        "<tr><td class=\"t-lbl\">صافي الخصم</td><td class=\"t-val\">{}</td></tr>",
        if discount > 0 { fmt_money(discount) } else { "0".to_owned() }
    );

    // ── build HTML ─────────────────────────────────────────────────────────
    let html = format!(
        r#"<!DOCTYPE html>
<html dir="rtl" lang="ar">
<head>
  <meta charset="utf-8">
  <title>فاتورة {inv_number}</title>
  <style>
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    @page {{ size: A4 portrait; margin: 10mm 12mm 10mm 12mm; }}
    body {{
      font-family: 'Arial', 'Tahoma', 'Segoe UI', sans-serif;
      font-size: 10pt; line-height: 1.45; color: #000;
      direction: rtl; background: #fff;
    }}
    .page {{ width: 100%; max-width: 186mm; margin: 0 auto; }}

    /* ── top header ── */
    .top-header {{
      display: flex; flex-direction: row; align-items: center;
      justify-content: space-between;
      border: 1.5pt solid #000; padding: 6pt 8pt; margin-bottom: 0;
    }}
    .h-logo {{ flex: 0 0 auto; }}
    .logo-img {{ max-width: 80pt; max-height: 55pt; width: auto; height: auto;
                 object-fit: contain; display: block; }}
    .h-shop {{ flex: 1 1 auto; text-align: center; }}
    .shop-name {{ font-size: 14pt; font-weight: bold; line-height: 1.3; }}
    .h-inv {{ flex: 0 0 auto; text-align: left; font-size: 9pt; }}

    /* ── meta section ── */
    .meta-section {{
      display: flex; flex-direction: row;
      border: 1.5pt solid #000; border-top: none;
    }}
    .meta-right {{ flex: 1 1 55%; border-left: 1pt solid #000; padding: 5pt 7pt; }}
    .meta-left  {{ flex: 1 1 45%; padding: 5pt 7pt; }}
    .meta-row   {{ display: flex; gap: 4pt; padding: 1.5pt 0; }}
    .mk {{ font-size: 9pt; font-weight: bold; white-space: nowrap; min-width: 60pt; }}
    .mv {{ font-size: 9pt; font-weight: bold; }}

    /* ── items table ── */
    .items-tbl {{
      width: 100%; border-collapse: collapse;
      font-size: 9.5pt;
      border: 1.5pt solid #000; border-top: none;
    }}
    .items-tbl thead tr {{ background: #ddd; }}
    .items-tbl th {{
      padding: 4pt; font-weight: bold; font-size: 9pt;
      border: 1pt solid #555; text-align: center;
    }}
    .c-num   {{ width: 5%;  text-align: center; }}
    .c-name  {{ width: 34%; text-align: right; padding-right: 5pt; word-break: break-word; }}
    .c-qty   {{ width: 8%;  text-align: center; }}
    .c-price {{ width: 13%; text-align: center; }}
    .c-disc  {{ width: 8%;  text-align: center; }}
    .c-net   {{ width: 14%; text-align: center; }}
    .c-total {{ width: 14%; text-align: center; }}
    .items-tbl td {{ padding: 3.5pt 4pt; border: 1pt solid #888; }}
    .row-e {{ background: #fff; }}
    .row-o {{ background: #f5f5f5; }}

    /* ── totals section ── */
    .totals-section {{
      display: flex; flex-direction: row;
      border: 1.5pt solid #000; border-top: none;
    }}
    .totals-left {{
      flex: 1 1 auto; padding: 4pt 6pt;
      display: flex; align-items: center;
      font-size: 9pt; border-left: 1pt solid #000;
    }}
    .paid-label {{ font-weight: bold; margin-left: 4pt; }}
    .totals-tbl {{ border-collapse: collapse; min-width: 180pt; }}
    .totals-tbl td {{ padding: 3pt 6pt; border: 1pt solid #888; font-size: 9.5pt; }}
    .t-lbl {{ font-weight: bold; text-align: right; white-space: nowrap; background: #eee; }}
    .t-val {{ text-align: center; font-weight: bold; min-width: 70pt; }}
    .grand-lbl {{ font-size: 10.5pt; font-weight: bold; background: #eee; }}
    .grand-val {{ font-size: 10.5pt; font-weight: bold; }}

    /* ── balance ── */
    .balance-section {{
      display: flex; align-items: center; gap: 6pt;
      border: 1.5pt solid #000; border-top: none;
      padding: 3pt 7pt; font-size: 9pt;
    }}
    .balance-label {{ font-weight: bold; }}
    .balance-value {{ font-weight: bold; }}
    .balance-dir   {{ font-weight: bold; font-size: 10pt;
                      border: 1pt solid #000; padding: 0 4pt; margin-right: 4pt; }}

    /* ── footer ── */
    .words-footer {{
      border: 1.5pt solid #000; border-top: none;
      padding: 4pt 7pt; font-size: 9.5pt; font-weight: bold;
    }}
    .shop-footer {{
      border-top: 1.5pt solid #000; margin-top: 8pt;
      padding-top: 4pt; text-align: center;
      font-size: 9pt; font-weight: bold;
    }}

    @media print {{
      body {{ margin: 0; }}
      .page {{ max-width: 100%; }}
      .items-tbl thead {{ display: table-header-group; }}
      tr {{ page-break-inside: avoid; }}
      .totals-section {{ page-break-inside: avoid; }}
      .words-footer {{ page-break-inside: avoid; }}
    }}
  </style>
  <script>window.addEventListener('load', function() {{ window.print(); }});</script>
</head>
<body>
<div class="page">

  <!-- TOP HEADER -->
  <div class="top-header">
    <div class="h-logo">{logo_html}</div>
    <div class="h-shop">
      <div class="shop-name">{shop_name_esc}</div>
      {address_html}
      {phone_html}
    </div>
    <div class="h-inv">
      <div style="font-size:9pt;font-weight:bold;">رقم الفاتورة</div>
      <div style="font-size:13pt;font-weight:bold;">{inv_number}</div>
    </div>
  </div>

  <!-- META SECTION -->
  <div class="meta-section">
    <div class="meta-right">
      <div class="meta-row"><span class="mk">اسم العميل :</span><span class="mv">{customer}</span></div>
      <div class="meta-row"><span class="mk">العنوان :</span><span class="mv"></span></div>
      <div class="meta-row"><span class="mk">الهاتف :</span><span class="mv"></span></div>
    </div>
    <div class="meta-left">
      <div class="meta-row"><span class="mk">المندوب :</span><span class="mv"></span></div>
      <div class="meta-row"><span class="mk">المستخدم :</span><span class="mv">{cashier}</span></div>
      <div class="meta-row"><span class="mk">التاريخ :</span><span class="mv">{date} &nbsp; {time}</span></div>
    </div>
  </div>

  <!-- ITEMS TABLE -->
  <table class="items-tbl">
    <thead>
      <tr>
        <th class="c-num">م</th>
        <th class="c-name">اسم الصنف</th>
        <th class="c-qty">كمية</th>
        <th class="c-price">سعر</th>
        <th class="c-disc">خصم%</th>
        <th class="c-net">صافي السعر</th>
        <th class="c-total">قيمة</th>
      </tr>
    </thead>
    <tbody>{item_rows}</tbody>
  </table>

  <!-- TOTALS -->
  <div class="totals-section">
    <div class="totals-left">
      <span class="paid-label">مدفوع نقدا</span>
      <span>{paid}</span>
    </div>
    <table class="totals-tbl">
      <tr><td class="t-lbl">الإجمالي</td><td class="t-val">{subtotal}</td></tr>
      {discount_row}
      <tr><td class="t-lbl grand-lbl">صافي الفاتورة :</td><td class="t-val grand-val">{net_total}</td></tr>
    </table>
  </div>

  {balance_html}

  <!-- FOOTER -->
  <div class="words-footer">{words}</div>
  <div class="shop-footer">شكرًا لثقتكم في {shop_name_esc}</div>

</div>
</body>
</html>"#,
        inv_number = esc(&invoice.invoice_number),
        shop_name_esc = esc(&shop_name),
        logo_html = logo_html,
        address_html = address_html,
        phone_html = phone_html,
        customer = esc(customer_str),
        cashier = esc(cashier_str),
        date = esc(&date_str),
        time = esc(&time_str),
        item_rows = item_rows,
        subtotal = fmt_money(subtotal),
        discount_row = discount_row,
        net_total = fmt_money(net_total),
        paid = fmt_money(paid),
        balance_html = balance_html,
        words = esc(&words),
    );

    // ── write file to persistent app-data directory ────────────────────────
    let out_dir = invoice_output_dir()?;
    let out_path = out_dir.join(format!("invoice_{}.html", invoice.id));
    std::fs::write(&out_path, html.as_bytes()).map_err(|e| {
        AppError::new(
            "INVOICE_WRITE_ERROR",
            "تعذر حفظ ملف الفاتورة",
            &format!("Failed to write invoice HTML: {e}"),
        )
    })?;

    Ok(out_path)
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// HTML-escape a string.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Format milliemes as a decimal money string (e.g. 3788120 → "3788.12").
fn fmt_money(millieme: i64) -> String {
    format!("{:.2}", millieme as f64 / 1000.0)
}

/// Split "YYYY-MM-DD HH:MM:SS" or ISO 8601 into (date, time) strings.
fn split_datetime(created_at: &str) -> (String, String) {
    let sep = if created_at.contains('T') { 'T' } else { ' ' };
    let mut parts = created_at.splitn(2, sep);
    let date = parts.next().unwrap_or(created_at).to_owned();
    let time = parts
        .next()
        .map(|t| t.split('+').next().unwrap_or(t).trim().to_owned())
        .unwrap_or_default();
    (date, time)
}

/// Read the logo file and embed it as a base64 data-URL.
/// Returns an empty string on any error (logo is non-fatal).
fn embed_logo(path: &str) -> String {
    let p = std::path::Path::new(path.trim());
    if !p.exists() {
        return String::new();
    }
    let bytes = match std::fs::read(p) {
        Ok(b) if !b.is_empty() => b,
        _ => return String::new(),
    };
    let ext = p
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
    let b64 = base64_encode(&bytes);
    format!(
        "<img src=\"data:{mime};base64,{b64}\" alt=\"شعار المعرض\" class=\"logo-img\" />"
    )
}

/// Minimal base64 encoder (no external dependencies).
fn base64_encode(data: &[u8]) -> String {
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

/// Returns (or creates) the persistent directory for generated invoice files.
fn invoice_output_dir() -> Result<PathBuf, AppError> {
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
            "INVOICE_DIR_ERROR",
            "تعذر إنشاء مجلد الفواتير",
            &format!("Failed to create invoices directory: {e}"),
        )
    })?;
    Ok(out_dir)
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

// ── Arabic amount-to-words ────────────────────────────────────────────────────

const ONES: &[&str] = &[
    "", "واحد", "اثنان", "ثلاثة", "أربعة", "خمسة", "ستة", "سبعة",
    "ثمانية", "تسعة", "عشرة", "أحد عشر", "اثنا عشر", "ثلاثة عشر",
    "أربعة عشر", "خمسة عشر", "ستة عشر", "سبعة عشر", "ثمانية عشر", "تسعة عشر",
];
const TENS: &[&str] = &[
    "", "عشرة", "عشرون", "ثلاثون", "أربعون", "خمسون",
    "ستون", "سبعون", "ثمانون", "تسعون",
];
const HUNDREDS: &[&str] = &[
    "", "مائة", "مئتان", "ثلاثمائة", "أربعمائة", "خمسمائة",
    "ستمائة", "سبعمائة", "ثمانمائة", "تسعمائة",
];

fn below_thousand(n: i64) -> String {
    if n == 0 {
        return String::new();
    }
    let mut parts: Vec<&str> = Vec::new();
    let h = (n / 100) as usize;
    let rem = n % 100;
    if h > 0 {
        parts.push(HUNDREDS[h]);
    }
    let mut extra = String::new();
    if rem >= 20 {
        let t = (rem / 10) as usize;
        let o = (rem % 10) as usize;
        if o > 0 {
            extra = format!("{} و{}", ONES[o], TENS[t]);
        } else {
            extra = TENS[t].to_owned();
        }
    } else if rem > 0 {
        extra = ONES[rem as usize].to_owned();
    }
    if !extra.is_empty() {
        if !parts.is_empty() {
            format!("{} و{}", parts.join(" و"), extra)
        } else {
            extra
        }
    } else {
        parts.join(" و")
    }
}

fn integer_to_arabic(n: i64) -> String {
    if n == 0 {
        return "صفر".to_owned();
    }
    let billions = n / 1_000_000_000;
    let millions = (n % 1_000_000_000) / 1_000_000;
    let thousands = (n % 1_000_000) / 1_000;
    let remainder = n % 1_000;

    let mut parts: Vec<String> = Vec::new();

    if billions > 0 {
        let s = match billions {
            1 => "مليار".to_owned(),
            2 => "ملياران".to_owned(),
            b if b <= 10 => format!("{} مليارات", below_thousand(b)),
            b => format!("{} مليار", below_thousand(b)),
        };
        parts.push(s);
    }
    if millions > 0 {
        let s = match millions {
            1 => "مليون".to_owned(),
            2 => "مليونان".to_owned(),
            m if m <= 10 => format!("{} ملايين", below_thousand(m)),
            m => format!("{} مليون", below_thousand(m)),
        };
        parts.push(s);
    }
    if thousands > 0 {
        let s = match thousands {
            1 => "ألف".to_owned(),
            2 => "ألفان".to_owned(),
            t if t <= 10 => format!("{} آلاف", below_thousand(t)),
            t => format!("{} ألف", below_thousand(t)),
        };
        parts.push(s);
    }
    if remainder > 0 {
        parts.push(below_thousand(remainder));
    }
    parts.join(" و")
}

/// Converts a millieme amount to Arabic words for printing on invoices.
/// e.g. 3_788_120 → "ثلاثة آلاف وسبعمائة وثمانية وثمانون جنيهاً و١٢ قرشاً فقط لا غير"
fn millieme_to_arabic_words(millieme: i64) -> String {
    if millieme <= 0 {
        return "صفر".to_owned();
    }
    let pounds = millieme / 1000;
    let piasters = ((millieme % 1000) as f64 / 10.0).round() as i64;

    let pounds_word = integer_to_arabic(pounds);
    let pounds_unit = if pounds == 1 { "جنيه" } else { "جنيهاً" };

    if piasters == 0 {
        return format!("{pounds_word} {pounds_unit} فقط لا غير");
    }

    // Convert piasters to Eastern Arabic numerals
    let piasters_ar: String = piasters
        .to_string()
        .chars()
        .map(|c| {
            "٠١٢٣٤٥٦٧٨٩"
                .chars()
                .nth(c.to_digit(10).unwrap_or(0) as usize)
                .unwrap_or(c)
        })
        .collect();

    format!("{pounds_word} {pounds_unit} و{piasters_ar} قرشاً فقط لا غير")
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_datetime_space_separator() {
        let (d, t) = split_datetime("2026-10-23 18:05:03");
        assert_eq!(d, "2026-10-23");
        assert_eq!(t, "18:05:03");
    }

    #[test]
    fn split_datetime_t_separator() {
        let (d, t) = split_datetime("2026-10-23T18:05:03+03:00");
        assert_eq!(d, "2026-10-23");
        assert_eq!(t, "18:05:03");
    }

    #[test]
    fn split_datetime_date_only() {
        let (d, t) = split_datetime("2026-10-23");
        assert_eq!(d, "2026-10-23");
        assert_eq!(t, "");
    }

    #[test]
    fn fmt_money_two_decimals() {
        assert_eq!(fmt_money(150_000), "150.00");
        assert_eq!(fmt_money(150_500), "150.50");
        assert_eq!(fmt_money(0), "0.00");
    }

    #[test]
    fn arabic_words_simple() {
        // 100 EGP = 100_000 milliemes
        let result = millieme_to_arabic_words(100_000);
        assert!(result.contains("مائة"), "got: {result}");
        assert!(result.contains("فقط لا غير"), "got: {result}");
    }

    #[test]
    fn arabic_words_with_piasters() {
        // 3788.12 EGP = 3_788_120 milliemes
        let result = millieme_to_arabic_words(3_788_120);
        assert!(result.contains("آلاف") || result.contains("ألف"), "got: {result}");
        assert!(result.contains("قرشاً"), "got: {result}");
    }

    #[test]
    fn setting_text_fallback() {
        let map = HashMap::new();
        assert_eq!(setting_text(&map, "shop_name", "fallback"), "fallback");
    }

    #[test]
    fn setting_text_whitespace_value() {
        let mut map = HashMap::new();
        map.insert("shop_name".to_owned(), "   ".to_owned());
        assert_eq!(setting_text(&map, "shop_name", "fallback"), "fallback");
    }

    #[test]
    fn base64_encode_roundtrip() {
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"M"), "TQ==");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b""), "");
    }

    #[test]
    fn esc_html_entities() {
        assert_eq!(esc("<b>test & \"val\"</b>"), "&lt;b&gt;test &amp; &quot;val&quot;&lt;/b&gt;");
    }
}
