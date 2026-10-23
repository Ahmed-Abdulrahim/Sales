import type {
  CreateReturnPayload,
  InvoiceDetail,
  InvoiceFilters,
  InvoicePrintData,
  InvoiceStats,
  InvoiceSummary,
  SaleReturn,
} from "@/modules/sales/types";
import { invoke } from "@/shared/utils/invoke";

export function getInvoiceStats() {
  return invoke<InvoiceStats>("get_invoice_stats");
}

export function listInvoices(filters: InvoiceFilters) {
  return invoke<InvoiceSummary[]>("list_invoices", { filters });
}

export function getInvoiceDetail(invoiceId: number | null) {
  return invoke<InvoiceDetail>("get_invoice_detail", { invoiceId });
}

export function createReturn(payload: CreateReturnPayload) {
  return invoke<SaleReturn>("create_return", { payload }, { toast: false });
}

export function getInvoicePrintData(invoiceId: number) {
  return invoke<InvoicePrintData>("get_invoice_print_data", { invoiceId }, { toast: false });
}

/** Reads a local logo file and returns it as a base64 data URL for embedding in HTML. */
export function readLogoBase64(logoPath: string) {
  return invoke<string>("read_logo_base64", { logoPath }, { toast: false });
}

/**
 * Copies the selected logo into the persistent app-data logos directory and
 * returns the managed path. Validates file type (PNG/JPEG/WEBP/GIF/BMP) and
 * size (max 5 MB).
 */
export function saveLogo(sourcePath: string) {
  return invoke<string>("save_logo", { sourcePath }, { toast: false });
}

/** Removes the managed logo file. Always succeeds (idempotent). */
export function deleteLogo() {
  return invoke<boolean>("delete_logo", {}, { toast: false });
}

/**
 * Fetches invoice print data and, if a logo path is configured, reads the
 * logo from disk and attaches it as a base64 data URL ready for HTML embedding.
 */
export async function getInvoicePrintDataWithLogo(invoiceId: number): Promise<InvoicePrintData> {
  const data = await getInvoicePrintData(invoiceId);
  if (data.shop.shopLogoPath) {
    try {
      const dataUrl = await readLogoBase64(data.shop.shopLogoPath);
      data.shopLogoDataUrl = dataUrl;
    } catch {
      // Logo load failure is non-fatal — print continues without logo
      data.shopLogoDataUrl = "";
    }
  }
  return data;
}

export function generateInvoicePdf(invoiceId: number) {
  return invoke<string>("generate_invoice_pdf", { invoiceId }, { toast: false });
}

export function openWhatsappWithInvoice({
  invoiceId,
  invoiceNumber,
}: {
  invoiceId: number;
  invoiceNumber: string;
}) {
  return invoke(
    "open_whatsapp_with_invoice",
    { invoiceId, invoiceNumber },
    { toast: false },
  );
}
