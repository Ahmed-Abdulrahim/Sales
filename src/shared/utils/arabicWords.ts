/**
 * Converts a non-negative number (Egyptian Pounds + piasters) to Arabic words.
 *
 * Examples:
 *   3788.12 → "ثلاثة آلاف وسبعمائة وثمانية وثمانون جنيهاً و١٢ قرشاً"
 *   100     → "مائة جنيه"
 *   0       → "صفر"
 */

const ONES: readonly string[] = [
  "",
  "واحد",
  "اثنان",
  "ثلاثة",
  "أربعة",
  "خمسة",
  "ستة",
  "سبعة",
  "ثمانية",
  "تسعة",
  "عشرة",
  "أحد عشر",
  "اثنا عشر",
  "ثلاثة عشر",
  "أربعة عشر",
  "خمسة عشر",
  "ستة عشر",
  "سبعة عشر",
  "ثمانية عشر",
  "تسعة عشر",
];

const TENS: readonly string[] = [
  "",
  "عشرة",
  "عشرون",
  "ثلاثون",
  "أربعون",
  "خمسون",
  "ستون",
  "سبعون",
  "ثمانون",
  "تسعون",
];

const HUNDREDS: readonly string[] = [
  "",
  "مائة",
  "مئتان",
  "ثلاثمائة",
  "أربعمائة",
  "خمسمائة",
  "ستمائة",
  "سبعمائة",
  "ثمانمائة",
  "تسعمائة",
];

/**
 * Converts an integer 0–999 to Arabic words.
 */
function belowThousand(n: number): string {
  if (n === 0) return "";

  const parts: string[] = [];

  const h = Math.floor(n / 100);
  const remainder = n % 100;

  if (h > 0) {
    parts.push(HUNDREDS[h]);
  }

  if (remainder >= 20) {
    const t = Math.floor(remainder / 10);
    const o = remainder % 10;
    if (o > 0) {
      parts.push(ONES[o] + " و" + TENS[t]);
    } else {
      parts.push(TENS[t]);
    }
  } else if (remainder > 0) {
    parts.push(ONES[remainder]);
  }

  return parts.join(" و");
}

/**
 * Converts a non-negative integer to Arabic words.
 * Handles up to 999,999,999,999.
 */
function integerToArabicWords(n: number): string {
  if (n === 0) return "صفر";
  if (!Number.isFinite(n) || n < 0) return "صفر";

  const billions = Math.floor(n / 1_000_000_000);
  const millions = Math.floor((n % 1_000_000_000) / 1_000_000);
  const thousands = Math.floor((n % 1_000_000) / 1_000);
  const remainder = n % 1_000;

  const parts: string[] = [];

  if (billions > 0) {
    if (billions === 1) {
      parts.push("مليار");
    } else if (billions === 2) {
      parts.push("ملياران");
    } else if (billions <= 10) {
      parts.push(belowThousand(billions) + " مليارات");
    } else {
      parts.push(belowThousand(billions) + " مليار");
    }
  }

  if (millions > 0) {
    if (millions === 1) {
      parts.push("مليون");
    } else if (millions === 2) {
      parts.push("مليونان");
    } else if (millions <= 10) {
      parts.push(belowThousand(millions) + " ملايين");
    } else {
      parts.push(belowThousand(millions) + " مليون");
    }
  }

  if (thousands > 0) {
    if (thousands === 1) {
      parts.push("ألف");
    } else if (thousands === 2) {
      parts.push("ألفان");
    } else if (thousands <= 10) {
      parts.push(belowThousand(thousands) + " آلاف");
    } else {
      parts.push(belowThousand(thousands) + " ألف");
    }
  }

  if (remainder > 0) {
    parts.push(belowThousand(remainder));
  }

  return parts.join(" و");
}

/**
 * Converts a money amount (in full pounds, e.g. 3788.12) to Arabic words
 * suitable for printing on Egyptian invoices.
 *
 * Returns a string like:
 *   "ثلاثة آلاف وسبعمائة وثمانية وثمانون جنيهاً و١٢ قرشاً فقط لا غير"
 */
export function moneyToArabicWords(amount: number): string {
  if (!Number.isFinite(amount) || amount < 0) return "صفر";

  // Round to 2 decimal places to avoid floating-point noise
  const rounded = Math.round(amount * 100) / 100;
  const pounds = Math.floor(rounded);
  const piasters = Math.round((rounded - pounds) * 100);

  if (pounds === 0 && piasters === 0) return "صفر";

  const poundsWord = integerToArabicWords(pounds);
  const poundsUnit = pounds === 1 ? "جنيه" : "جنيهاً";

  if (piasters === 0) {
    return `${poundsWord} ${poundsUnit} فقط لا غير`;
  }

  // Convert piasters to Eastern Arabic numerals
  const piastersAr = String(piasters).replace(
    /\d/g,
    (d) => "٠١٢٣٤٥٦٧٨٩"[Number(d)],
  );

  return `${poundsWord} ${poundsUnit} و${piastersAr} قرشاً فقط لا غير`;
}

/**
 * Converts a millieme amount (1/1000 of a pound) to Arabic words.
 *
 * @param millieme - Amount in milliemes (e.g. 3788120 → 3788.12 EGP)
 */
export function milliemeToArabicWords(millieme: number): string {
  return moneyToArabicWords(millieme / 1000);
}
