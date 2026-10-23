import type { InvoicePrintData } from "@/modules/sales/types";
import { formatEGP } from "@/shared/utils/money";

/**
 * Builds a professional A4 RTL printable HTML invoice for:
 * معرض أولاد عبد الرحيم للأدوات الصحية
 *
 * Shared by the POS success screen and the invoice detail sheet.
 * Pair with `printHtml` from `@/shared/utils/printHtml` to send to print.
 *
 * The `data.shopLogoDataUrl` field (optional) should be pre-populated with a
 * base64 data URL before calling this function; use `getInvoicePrintDataWithLogo`
 * from the relevant api module rather than `getInvoicePrintData` directly.
 */
export function buildInvoicePrintHtml(data: InvoicePrintData): string {
  /* ── helpers ── */
  function esc(s: string): string {
    return s
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  function fmtDate(iso: string): string {
    const norm = iso.includes("T") ? iso : iso.replace(" ", "T");
    const d = new Date(norm);
    if (isNaN(d.getTime())) return iso;
    return new Intl.DateTimeFormat("ar-EG", { dateStyle: "medium" }).format(d);
  }

  function fmtTime(iso: string): string {
    const norm = iso.includes("T") ? iso : iso.replace(" ", "T");
    const d = new Date(norm);
    if (isNaN(d.getTime())) return "";
    return new Intl.DateTimeFormat("ar-EG", { timeStyle: "short" }).format(d);
  }

  /* ── header content ── */
  const shopName = esc(data.shop.shopName || "معرض أولاد عبد الرحيم للأدوات الصحية");

  const logoHtml =
    data.shopLogoDataUrl
      ? `<img src="${data.shopLogoDataUrl}" alt="شعار المعرض" class="logo-img" />`
      : "";

  // Optional shop contact lines — only show when non-empty and not the default placeholder
  const addressVal = (data.shop.shopAddress || "").trim();
  const phoneVal = (data.shop.shopPhone || "").trim();
  const addressHtml =
    addressVal && addressVal !== "العنوان" && addressVal !== "العنوان غير محدد"
      ? `<div class="contact-line">${esc(addressVal)}</div>`
      : "";
  const phoneHtml = phoneVal
    ? `<div class="contact-line">${esc(phoneVal)}</div>`
    : "";

  /* ── metadata ── */
  const customerLabel = data.customerName
    ? esc(data.customerName)
    : '<span class="na">عميل عام</span>';
  const cashierLabel = data.cashierName ? esc(data.cashierName) : "";
  const dateStr = esc(fmtDate(data.createdAt));
  const timeStr = esc(fmtTime(data.createdAt));

  /* ── items rows ── */
  const itemsHtml = data.items
    .map(
      (item, i) => `
      <tr class="${i % 2 === 0 ? "row-e" : "row-o"}">
        <td class="c-num">${i + 1}</td>
        <td class="c-name">${esc(item.itemNameAr)}</td>
        <td class="c-qty">${item.qty}</td>
        <td class="c-price">${esc(formatEGP(item.unitPriceMillieme))}</td>
        <td class="c-total">${esc(formatEGP(item.totalMillieme))}</td>
      </tr>`,
    )
    .join("");

  /* ── discount row (only shown when non-zero) ── */
  const discountRowHtml =
    data.discountMillieme > 0
      ? `<tr class="t-row">
           <td class="t-lbl">الخصم</td>
           <td class="t-val">${esc(formatEGP(data.discountMillieme))}</td>
         </tr>`
      : "";

  /* ── footer message ── */
  const footerMsg = "شكرًا لثقتكم في معرض أولاد عبد الرحيم للأدوات الصحية";

  return `<!DOCTYPE html>
<html dir="rtl" lang="ar">
<head>
  <meta charset="utf-8">
  <title>فاتورة ${esc(data.invoiceNumber)}</title>
  <style>
    /* ── reset ── */
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

    @page {
      size: A4 portrait;
      margin: 14mm 14mm 14mm 14mm;
    }

    /* ── base ── */
    body {
      font-family: 'Arial', 'Tahoma', 'Segoe UI', sans-serif;
      font-size: 10.5pt;
      line-height: 1.5;
      color: #111;
      direction: rtl;
      background: #fff;
    }

    .page {
      width: 100%;
      max-width: 182mm;
      margin: 0 auto;
    }

    /* ── HEADER ── */
    .inv-header {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      padding-bottom: 9pt;
      border-bottom: 2.5pt solid #222;
      margin-bottom: 8pt;
    }

    .h-logo {
      flex: 0 0 auto;
      min-width: 0;
    }

    .logo-img {
      max-width: 88pt;
      max-height: 58pt;
      width: auto;
      height: auto;
      object-fit: contain;
      display: block;
    }

    .h-shop {
      flex: 1 1 auto;
      text-align: right;
      padding-right: 8pt;
    }

    .shop-name {
      font-size: 13.5pt;
      font-weight: bold;
      color: #111;
      line-height: 1.3;
    }

    .contact-line {
      font-size: 9pt;
      color: #555;
      margin-top: 2pt;
    }

    /* ── INVOICE-ID BAR ── */
    .id-bar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      background: #f4f4f4;
      border: 1pt solid #ccc;
      border-radius: 3pt;
      padding: 5pt 9pt;
      margin-bottom: 8pt;
    }

    .id-lbl { font-size: 9pt; color: #555; font-weight: bold; }
    .id-num { font-size: 13pt; font-weight: bold; color: #111; letter-spacing: 0.4pt; }

    /* ── META GRID ── */
    .meta-box {
      border: 1pt solid #ddd;
      border-radius: 3pt;
      margin-bottom: 10pt;
    }

    .meta-row {
      display: flex;
      flex-direction: row;
      align-items: stretch;
      border-bottom: 1pt solid #eee;
    }

    .meta-row:last-child { border-bottom: none; }

    .meta-cell {
      flex: 1 1 50%;
      display: flex;
      align-items: center;
      gap: 5pt;
      padding: 5pt 8pt;
    }

    .meta-cell + .meta-cell { border-right: 1pt solid #eee; }

    .mk { font-size: 9pt; color: #555; font-weight: bold; white-space: nowrap; }
    .mv { font-size: 10pt; font-weight: bold; color: #111; }
    .na { font-weight: normal; color: #888; }

    /* ── ITEMS TABLE ── */
    .items-tbl {
      width: 100%;
      border-collapse: collapse;
      margin-bottom: 10pt;
      font-size: 10pt;
    }

    .items-tbl thead tr {
      background: #222;
      color: #fff;
    }

    .items-tbl th {
      padding: 5pt 6pt;
      font-weight: bold;
      font-size: 9.5pt;
      border: 1pt solid #222;
      text-align: center;
    }

    /* RTL column order: م | اسم الصنف | الكمية | سعر الوحدة | الإجمالي */
    .c-num   { width: 6%;  text-align: center; }
    .c-name  { width: 46%; text-align: right;  word-break: break-word; }
    .c-qty   { width: 10%; text-align: center; }
    .c-price { width: 19%; text-align: center; }
    .c-total { width: 19%; text-align: center; }

    .items-tbl td {
      padding: 5pt 6pt;
      border: 1pt solid #ddd;
    }

    .row-e { background: #fff; }
    .row-o { background: #fafafa; }

    /* ── TOTALS ── */
    .totals-wrap {
      display: flex;
      justify-content: flex-start;
      margin-bottom: 16pt;
    }

    .totals-tbl {
      border-collapse: collapse;
      width: 220pt;
      font-size: 10.5pt;
    }

    .totals-tbl td {
      padding: 4pt 8pt;
      border: 1pt solid #ddd;
    }

    .t-lbl { font-weight: bold; color: #444; text-align: right; white-space: nowrap; }
    .t-val { text-align: center; font-weight: bold; color: #111; }

    .grand-row td {
      background: #222;
      color: #fff;
      font-size: 12pt;
      font-weight: bold;
      border-color: #222;
    }

    /* ── FOOTER ── */
    .inv-footer {
      border-top: 2pt solid #222;
      padding-top: 8pt;
      text-align: center;
      font-size: 10pt;
      font-weight: bold;
      color: #222;
    }

    /* ── PRINT ── */
    @media print {
      body { margin: 0; }
      .page { max-width: 100%; }
      .items-tbl thead { display: table-header-group; }
      tr { page-break-inside: avoid; }
      .totals-wrap { page-break-inside: avoid; }
      .inv-footer  { page-break-inside: avoid; }
    }
  </style>
</head>
<body>
  <div class="page">

    <!-- HEADER: logo (left) | shop name (right) -->
    <div class="inv-header">
      <div class="h-logo">${logoHtml}</div>
      <div class="h-shop">
        <div class="shop-name">${shopName}</div>
        ${addressHtml}${phoneHtml}
      </div>
    </div>

    <!-- INVOICE NUMBER BAR -->
    <div class="id-bar">
      <span class="id-lbl">رقم الفاتورة</span>
      <span class="id-num">${esc(data.invoiceNumber)}</span>
    </div>

    <!-- METADATA -->
    <div class="meta-box">
      <div class="meta-row">
        <div class="meta-cell">
          <span class="mk">العميل:</span>
          <span class="mv">${customerLabel}</span>
        </div>
        <div class="meta-cell">
          ${cashierLabel ? `<span class="mk">البائع:</span><span class="mv">${cashierLabel}</span>` : ""}
        </div>
      </div>
      <div class="meta-row">
        <div class="meta-cell">
          <span class="mk">التاريخ:</span>
          <span class="mv">${dateStr}</span>
        </div>
        <div class="meta-cell">
          <span class="mk">الوقت:</span>
          <span class="mv">${timeStr}</span>
        </div>
      </div>
    </div>

    <!-- ITEMS TABLE  (RTL column order: م | اسم الصنف | الكمية | سعر الوحدة | الإجمالي) -->
    <table class="items-tbl">
      <thead>
        <tr>
          <th class="c-num">م</th>
          <th class="c-name">اسم الصنف</th>
          <th class="c-qty">الكمية</th>
          <th class="c-price">سعر الوحدة</th>
          <th class="c-total">الإجمالي</th>
        </tr>
      </thead>
      <tbody>${itemsHtml}</tbody>
    </table>

    <!-- TOTALS -->
    <div class="totals-wrap">
      <table class="totals-tbl">
        <tr class="t-row">
          <td class="t-lbl">الإجمالي قبل الخصم</td>
          <td class="t-val">${esc(formatEGP(data.subtotalMillieme))}</td>
        </tr>
        ${discountRowHtml}
        <tr class="grand-row">
          <td class="t-lbl">صافي الفاتورة</td>
          <td class="t-val">${esc(formatEGP(data.totalMillieme))}</td>
        </tr>
      </table>
    </div>

    <!-- FOOTER -->
    <div class="inv-footer">${esc(footerMsg)}</div>

  </div>
</body>
</html>`;
}
