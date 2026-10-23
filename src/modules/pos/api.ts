import type { Category, Item } from "@/modules/items/types";
import type { Customer } from "@/modules/parties/types";
import type { SaleInvoiceSuccess } from "@/modules/pos/InvoiceSuccessDialog";
import type { CreateSaleInvoicePayload } from "@/modules/pos/types";
import type { InvoicePrintData } from "@/modules/sales/types";
import { invoke } from "@/shared/utils/invoke";

export function listCategories() {
  return invoke<Category[]>("list_categories", undefined, { toast: false });
}

export function searchPosItems({
  query,
  categoryId,
}: {
  query: string | null;
  categoryId: number | null;
}) {
  return invoke<Item[]>(
    "search_items",
    { query, categoryId },
    { toast: false },
  );
}

export function searchCustomers(search: string | null) {
  return invoke<Customer[]>("list_customers", { search }, { toast: false });
}

export function createSaleInvoice(payload: CreateSaleInvoicePayload) {
  return invoke<SaleInvoiceSuccess>(
    "create_sale_invoice",
    { payload },
    { toast: false },
  );
}

export function getInvoicePrintData(invoiceId: number) {
  return invoke<InvoicePrintData>("get_invoice_print_data", { invoiceId }, { toast: false });
}

/** Reads a local logo file and returns it as a base64 data URL. */
export function readLogoBase64(logoPath: string) {
  return invoke<string>("read_logo_base64", { logoPath }, { toast: false });
}

/**
 * Fetches invoice print data and attaches the logo as a base64 data URL.
 */
export async function getInvoicePrintDataWithLogo(invoiceId: number): Promise<InvoicePrintData> {
  const data = await getInvoicePrintData(invoiceId);
  if (data.shop.shopLogoPath) {
    try {
      const dataUrl = await readLogoBase64(data.shop.shopLogoPath);
      data.shopLogoDataUrl = dataUrl;
    } catch {
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
