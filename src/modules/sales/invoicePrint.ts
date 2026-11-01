import type { InvoicePrintData } from "@/modules/sales/types";
import { formatEGP } from "@/shared/utils/money";
import { milliemeToArabicWords } from "@/shared/utils/arabicWords";

/**
 * Builds a traditional Arabic A4 sales invoice for:
 * معرض أولاد عبد الرحيم للأدوات الصحية
 *
 * Layout matches the reference paper invoice:
 *   - Header: shop name + logo top | customer info (right) | cashier/date (left)
 *   - Items table (RTL 7 cols): م | اسم الصنف | كمية | سعر | خصم% | صافي السعر | قيمة
 *   - Totals: الإجمالي | صافي الخصم | صافي الفاتورة | مدفوع نقدا | الرصيد الحالي
 *   - Footer: amount in Arabic words + "فقط لا غير"
 *
 * Use `getInvoicePrintDataWithLogo` (from the relevant api module) to pre-populate
 * `data.shopLogoDataUrl` before calling this function.
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
    // Format: YYYY-MM-DD
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return `${year}-${month}-${day}`;
  }

  function fmtTime(iso: string): string {
    const norm = iso.includes("T") ? iso : iso.replace(" ", "T");
    const d = new Date(norm);
    if (isNaN(d.getTime())) return "";
    // Format: HH:MM with Arabic AM/PM
    const hours = d.getHours();
    const minutes = String(d.getMinutes()).padStart(2, "0");
    const ampm = hours < 12 ? "ص" : "م";
    const h12 = hours % 12 || 12;
    return `${h12}:${minutes}.${ampm}`;
  }

  function fmt(millieme: number): string {
    return esc(formatEGP(millieme));
  }

  /* ── shop info ── */
  const shopName = esc(
    data.shop.shopName || "معرض أولاد عبد الرحيم للأدوات الصحية",
  );

  const logoHtml = data.shopLogoDataUrl
    ? `<img src="${data.shopLogoDataUrl}" alt="شعار المعرض" class="logo-img" />`
    : "";

  const addressVal = (data.shop.shopAddress || "").trim();
  const phoneVal = (data.shop.shopPhone || "").trim();
  const showAddress =
    addressVal &&
    addressVal !== "العنوان" &&
    addressVal !== "العنوان غير محدد";

  /* ── invoice metadata ── */
  const customerName = data.customerName
    ? esc(data.customerName)
    : '<span class="na">عميل عام</span>';
  const customerPhone = data.customerPhone ? esc(data.customerPhone) : null;
  const cashierName = data.cashierName ? esc(data.cashierName) : "—";
  const dateStr = esc(fmtDate(data.createdAt));
  const timeStr = esc(fmtTime(data.createdAt));

  /* ── per-item calculations ── */
  // Derive discount percentage and net unit price from milliemes.
  // discount_millieme is the total discount for the whole line (not per unit).
  const itemRows = data.items.map((item, i) => {
    const lineTotal = item.totalMillieme; // already net of discount
    const grossLine = item.qty * item.unitPriceMillieme;
    const discountTotal = grossLine - lineTotal; // total discount on the line
    const discountPct =
      grossLine > 0 ? Math.round((discountTotal / grossLine) * 100) : 0;
    const netUnitMillieme =
      item.qty > 0
        ? Math.round(lineTotal / item.qty)
        : item.unitPriceMillieme;

    return `
      <tr class="${i % 2 === 0 ? "row-e" : "row-o"}">
        <td class="c-num">${i + 1}</td>
        <td class="c-name">${esc(item.itemNameAr)}</td>
        <td class="c-qty">${item.qty}</td>
        <td class="c-price">${fmt(item.unitPriceMillieme)}</td>
        <td class="c-disc">${discountPct}</td>
        <td class="c-net">${fmt(netUnitMillieme)}</td>
        <td class="c-total">${fmt(lineTotal)}</td>
      </tr>`;
  });

  /* ── totals section ── */
  const subtotal = data.subtotalMillieme;
  const discount = data.discountMillieme;
  const netTotal = data.totalMillieme;
  const paid = data.paidMillieme;

  const customerBalance = data.customerBalanceMillieme;
  const balanceLabel =
    customerBalance != null && customerBalance > 0
      ? "عليه"
      : customerBalance != null && customerBalance < 0
        ? "له"
        : "";
  const balanceAbs =
    customerBalance != null ? Math.abs(customerBalance) : null;

  /* ── amount in words ── */
  const amountWords = esc(milliemeToArabicWords(netTotal));

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
      margin: 10mm 12mm 10mm 12mm;
    }

    /* ── base ── */
    body {
      font-family: 'Arial', 'Tahoma', 'Segoe UI', sans-serif;
      font-size: 10pt;
      line-height: 1.45;
      color: #000;
      direction: rtl;
      background: #fff;
    }

    .page {
      width: 100%;
      max-width: 186mm;
      margin: 0 auto;
    }

    /* ═══════════════════════════════════════════
       TOP HEADER: logo + shop name
    ═══════════════════════════════════════════ */
    .top-header {
      display: flex;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
      border: 1.5pt solid #000;
      padding: 6pt 8pt;
      margin-bottom: 0;
    }

    .h-logo {
      flex: 0 0 auto;
      min-width: 0;
    }

    .logo-img {
      max-width: 80pt;
      max-height: 55pt;
      width: auto;
      height: auto;
      object-fit: contain;
      display: block;
    }

    .h-shop {
      flex: 1 1 auto;
      text-align: center;
    }

    .shop-name {
      font-size: 14pt;
      font-weight: bold;
      color: #000;
      line-height: 1.3;
    }

    .inv-num-line {
      font-size: 10pt;
      font-weight: bold;
      margin-top: 3pt;
    }

    .h-inv {
      flex: 0 0 auto;
      text-align: left;
      font-size: 9pt;
    }

    /* ═══════════════════════════════════════════
       META SECTION: customer info (right) | cashier/date (left)
    ═══════════════════════════════════════════ */
    .meta-section {
      display: flex;
      flex-direction: row;
      border: 1.5pt solid #000;
      border-top: none;
      margin-bottom: 0;
    }

    .meta-right {
      flex: 1 1 55%;
      border-left: 1pt solid #000;
      padding: 5pt 7pt;
    }

    .meta-left {
      flex: 1 1 45%;
      padding: 5pt 7pt;
    }

    .meta-row {
      display: flex;
      flex-direction: row;
      align-items: baseline;
      gap: 4pt;
      padding: 1.5pt 0;
    }

    .mk {
      font-size: 9pt;
      font-weight: bold;
      color: #000;
      white-space: nowrap;
      min-width: 60pt;
    }

    .mv {
      font-size: 9pt;
      font-weight: bold;
      color: #000;
    }

    .na { font-weight: normal; color: #000; }

    /* ═══════════════════════════════════════════
       ITEMS TABLE
    ═══════════════════════════════════════════ */
    .items-tbl {
      width: 100%;
      border-collapse: collapse;
      margin-bottom: 0;
      font-size: 9.5pt;
      border: 1.5pt solid #000;
      border-top: none;
    }

    .items-tbl thead tr {
      background: #ddd;
    }

    .items-tbl th {
      padding: 4pt 4pt;
      font-weight: bold;
      font-size: 9pt;
      border: 1pt solid #000;
      text-align: center;
    }

    /* RTL column order */
    .c-num   { width: 5%;  text-align: center; }
    .c-name  { width: 35%; text-align: right; word-break: break-word; padding-right: 5pt; }
    .c-qty   { width: 8%;  text-align: center; }
    .c-price { width: 13%; text-align: center; }
    .c-disc  { width: 8%;  text-align: center; }
    .c-net   { width: 13%; text-align: center; }
    .c-total { width: 14%; text-align: center; }

    .items-tbl td {
      padding: 3.5pt 4pt;
      border: 1pt solid #000;
    }

    .row-e { background: #fff; }
    .row-o { background: #f5f5f5; }

    /* ═══════════════════════════════════════════
       TOTALS SECTION
    ═══════════════════════════════════════════ */
    .totals-section {
      display: flex;
      flex-direction: row;
      border: 1.5pt solid #000;
      border-top: none;
    }

    .totals-right {
      flex: 1 1 auto;
      padding: 4pt 6pt;
      display: flex;
      align-items: center;
      font-size: 9pt;
      border-left: 1pt solid #000;
    }

    .totals-right span {
      font-weight: bold;
    }

    .paid-label {
      font-weight: bold;
      margin-left: 4pt;
    }

    .totals-tbl {
      border-collapse: collapse;
      min-width: 180pt;
    }

    .totals-tbl td {
      padding: 3pt 6pt;
      border: 1pt solid #000;
      font-size: 9.5pt;
    }

    .t-lbl {
      font-weight: bold;
      text-align: right;
      white-space: nowrap;
      color: #000;
      background: #eee;
    }

    .t-val {
      text-align: center;
      font-weight: bold;
      color: #000;
      min-width: 70pt;
    }

    .grand-lbl {
      font-weight: bold;
      font-size: 10.5pt;
      background: #eee;
    }

    .grand-val {
      font-weight: bold;
      font-size: 10.5pt;
    }

    .balance-section {
      display: flex;
      flex-direction: row;
      align-items: center;
      gap: 6pt;
      border: 1.5pt solid #000;
      border-top: none;
      padding: 3pt 7pt;
      font-size: 9pt;
    }

    .balance-label { font-weight: bold; }
    .balance-value { font-weight: bold; }
    .balance-dir   { font-weight: bold; font-size: 10pt; border: 1pt solid #000; padding: 0 4pt; margin-right: 4pt; }

    /* ═══════════════════════════════════════════
       FOOTER — amount in words
    ═══════════════════════════════════════════ */
    .words-footer {
      border: 1.5pt solid #000;
      border-top: none;
      padding: 4pt 7pt;
      font-size: 9.5pt;
      font-weight: bold;
    }

    .shop-footer {
      border-top: 1.5pt solid #000;
      margin-top: 8pt;
      padding-top: 4pt;
      text-align: center;
      font-size: 9pt;
      font-weight: bold;
    }

    /* ── print ── */
    @media print {
      body { margin: 0; }
      .page { max-width: 100%; }
      .items-tbl thead { display: table-header-group; }
      tr { page-break-inside: avoid; }
      .totals-section { page-break-inside: avoid; }
      .words-footer { page-break-inside: avoid; }
    }
  </style>
</head>
<body>
  <div class="page">

    <!-- TOP HEADER: logo | shop name | invoice number -->
    <div class="top-header">
      <div class="h-logo">${logoHtml}</div>
      <div class="h-shop">
        <div class="shop-name">${shopName}</div>
        ${showAddress ? `<div style="font-size:8.5pt;margin-top:2pt;">${esc(addressVal)}</div>` : ""}
        ${phoneVal ? `<div style="font-size:8.5pt;">${esc(phoneVal)}</div>` : ""}
      </div>
      <div class="h-inv">
        <div style="font-size:9pt;font-weight:bold;">رقم الفاتورة</div>
        <div style="font-size:13pt;font-weight:bold;letter-spacing:0.5pt;">${esc(data.invoiceNumber)}</div>
      </div>
    </div>

    <!-- META SECTION -->
    <div class="meta-section">
      <!-- Right: customer info -->
      <div class="meta-right">
        <div class="meta-row">
          <span class="mk">اسم العميل :</span>
          <span class="mv">${customerName}</span>
        </div>
        <div class="meta-row">
          <span class="mk">الهاتف :</span>
          <span class="mv">${customerPhone ? customerPhone : '<span class="na"></span>'}</span>
        </div>
        <div class="meta-row">
          <span class="mk">العنوان :</span>
          <span class="mv na"></span>
        </div>
      </div>
      <!-- Left: cashier / date -->
      <div class="meta-left">
        <div class="meta-row">
          <span class="mk">المندوب :</span>
          <span class="mv na"></span>
        </div>
        <div class="meta-row">
          <span class="mk">المستخدم :</span>
          <span class="mv">${cashierName}</span>
        </div>
        <div class="meta-row">
          <span class="mk">التاريخ :</span>
          <span class="mv">${dateStr} &nbsp; ${timeStr}</span>
        </div>
      </div>
    </div>

    <!-- ITEMS TABLE (7 RTL columns) -->
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
      <tbody>${itemRows.join("")}</tbody>
    </table>

    <!-- TOTALS SECTION -->
    <div class="totals-section">
      <!-- Left: paid amount -->
      <div class="totals-right">
        <span class="paid-label">مدفوع نقدا</span>
        <span>${fmt(paid)}</span>
      </div>
      <!-- Right: totals table -->
      <table class="totals-tbl">
        <tr>
          <td class="t-lbl">الإجمالي</td>
          <td class="t-val">${fmt(subtotal)}</td>
        </tr>
        ${
          discount > 0
            ? `<tr>
          <td class="t-lbl">صافي الخصم</td>
          <td class="t-val">${fmt(discount)}</td>
        </tr>`
            : `<tr>
          <td class="t-lbl">صافي الخصم</td>
          <td class="t-val">0</td>
        </tr>`
        }
        <tr>
          <td class="t-lbl grand-lbl">صافي الفاتورة :</td>
          <td class="t-val grand-val">${fmt(netTotal)}</td>
        </tr>
      </table>
    </div>

    ${
      customerBalance != null && balanceAbs != null
        ? `<div class="balance-section">
      <span class="balance-label">الرصيد الحالي :</span>
      <span class="balance-value">${fmt(balanceAbs)}</span>
      ${balanceLabel ? `<span class="balance-dir">${esc(balanceLabel)}</span>` : ""}
    </div>`
        : ""
    }

    <!-- FOOTER: amount in words -->
    <div class="words-footer">${amountWords}</div>

    <!-- Shop footer line -->
    <div class="shop-footer">شكرًا لثقتكم في ${shopName}</div>

  </div>
</body>
</html>`;
}
