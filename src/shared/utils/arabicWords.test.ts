import { describe, expect, it } from "vitest";
import { moneyToArabicWords, milliemeToArabicWords } from "@/shared/utils/arabicWords";

describe("moneyToArabicWords", () => {
  it("returns صفر for zero", () => {
    expect(moneyToArabicWords(0)).toBe("صفر");
  });

  it("handles whole pounds with no piasters", () => {
    const result = moneyToArabicWords(100);
    expect(result).toContain("مائة");
    expect(result).toContain("فقط لا غير");
    expect(result).not.toContain("قرشاً");
  });

  it("handles whole pounds — single pound", () => {
    const result = moneyToArabicWords(1);
    expect(result).toContain("واحد");
    expect(result).toContain("جنيه");
  });

  it("handles piasters correctly", () => {
    // 3788.12 → should mention آلاف and قرشاً
    const result = moneyToArabicWords(3788.12);
    expect(result).toMatch(/آلاف|ألف/);
    expect(result).toContain("قرشاً");
    expect(result).toContain("فقط لا غير");
  });

  it("handles amounts in the thousands", () => {
    const result = moneyToArabicWords(1000);
    expect(result).toContain("ألف");
  });

  it("handles two thousand", () => {
    const result = moneyToArabicWords(2000);
    expect(result).toContain("ألفان");
  });

  it("handles millions", () => {
    const result = moneyToArabicWords(1_000_000);
    expect(result).toContain("مليون");
  });
});

describe("milliemeToArabicWords", () => {
  it("converts milliemes to Arabic words correctly", () => {
    // 3788120 milliemes = 3788.12 EGP
    const result = milliemeToArabicWords(3_788_120);
    expect(result).toMatch(/آلاف|ألف/);
    expect(result).toContain("قرشاً");
    expect(result).toContain("فقط لا غير");
  });

  it("handles exact pound amounts", () => {
    // 50000 milliemes = 50 EGP
    const result = milliemeToArabicWords(50_000);
    expect(result).toContain("خمسون");
    expect(result).not.toContain("قرشاً");
  });

  it("handles zero milliemes", () => {
    expect(milliemeToArabicWords(0)).toBe("صفر");
  });
});
