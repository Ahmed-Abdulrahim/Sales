import { useEffect, useRef, useState } from "react";
import { toast } from "sonner";

import { Card, CardContent } from "@/components/ui/card";
import { CatalogPanel } from "@/modules/pos/components/CatalogPanel";
import { CartTable } from "@/modules/pos/components/CartTable";
import { CustomerPanel } from "@/modules/pos/components/CustomerPanel";
import { InvoiceActionsPanel } from "@/modules/pos/components/InvoiceActionsPanel";
import { InvoiceItemAdjustDialog } from "@/modules/pos/components/InvoiceItemAdjustDialog";
import {
  InvoiceSuccessDialog,
  type SaleInvoiceSuccess,
} from "@/modules/pos/InvoiceSuccessDialog";
import { PaymentPanel } from "@/modules/pos/components/PaymentPanel";
import { TotalsPanel } from "@/modules/pos/components/TotalsPanel";
import {
  ensureCashPaidAtLeastTotal,
  shouldSyncCashPaidToTotal,
} from "@/modules/pos/paymentRules";
import {
  generateInvoicePdf,
  getInvoicePrintDataWithLogo,
  openWhatsappWithInvoice,
} from "@/modules/pos/api";
import type { Item } from "@/modules/items/types";
import { parseAppError } from "@/modules/items/utils";
import {
  useCreateSaleInvoice,
  usePosCategories,
  usePosItems,
} from "@/modules/pos/hooks";
import type { PaymentMethod } from "@/modules/pos/types";
import { buildSalePayload } from "@/modules/pos/utils";
import { useDebouncedValue } from "@/shared/hooks/useDebouncedValue";
import { buildInvoicePrintHtml } from "@/modules/sales/invoicePrint";
import { useInvalidate } from "@/shared/hooks/useInvalidate";
import { formatEGP } from "@/shared/utils/money";
import { printHtml } from "@/shared/utils/printHtml";
import { useCartStore } from "@/store/cartSlice";
import { useSessionStore } from "@/store/sessionSlice";
import { Button } from "@/components/ui/button";
import { OpenSessionDialog } from "@/modules/sessions/OpenSessionDialog";

export default function PosPage() {
  const invalidate = useInvalidate();
  const activeSession = useSessionStore((state) => state.activeSession);
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const itemClickTimeoutRef = useRef<number | null>(null);

  const cartItems = useCartStore((state) => state.items);
  const customerName = useCartStore((state) => state.customerName);
  const customerPhone = useCartStore((state) => state.customerPhone);
  const globalDiscountMillieme = useCartStore(
    (state) => state.globalDiscountMillieme,
  );
  const paymentMethod = useCartStore((state) => state.paymentMethod);
  const paidCashMillieme = useCartStore((state) => state.paidCashMillieme);
  const paidCashManuallySet = useCartStore(
    (state) => state.paidCashManuallySet,
  );
  const paidCardMillieme = useCartStore((state) => state.paidCardMillieme);
  const notes = useCartStore((state) => state.notes);
  const subtotalMillieme = useCartStore((state) => state.subtotalMillieme());
  const totalDiscountMillieme = useCartStore((state) =>
    state.totalDiscountMillieme(),
  );
  const totalMillieme = useCartStore((state) => state.totalMillieme());
  const changeMillieme = useCartStore((state) => state.changeMillieme());
  const remainingMillieme = useCartStore((state) => state.remainingMillieme());

  const addItem = useCartStore((state) => state.addItem);
  const removeItem = useCartStore((state) => state.removeItem);
  const updateQty = useCartStore((state) => state.updateQty);
  const updateLineDiscountPercent = useCartStore(
    (state) => state.updateLineDiscountPercent,
  );
  const updateLineUnitPrice = useCartStore(
    (state) => state.updateLineUnitPrice,
  );
  const setGlobalDiscount = useCartStore((state) => state.setGlobalDiscount);
  const setCustomerName = useCartStore((state) => state.setCustomerName);
  const setCustomerPhone = useCartStore((state) => state.setCustomerPhone);
  const setPaymentMethod = useCartStore((state) => state.setPaymentMethod);
  const setPaidCashAmount = useCartStore((state) => state.setPaidCashAmount);
  const setPaidCardAmount = useCartStore((state) => state.setPaidCardAmount);
  const setNotes = useCartStore((state) => state.setNotes);
  const clearCart = useCartStore((state) => state.clearCart);

  const [search, setSearch] = useState("");
  const [selectedCategoryId, setSelectedCategoryId] = useState<number | null>(
    null,
  );
  const [adjustedItem, setAdjustedItem] = useState<Item | null>(null);
  const [isAdjustDialogOpen, setIsAdjustDialogOpen] = useState(false);
  const [openSessionDialogOpen, setOpenSessionDialogOpen] = useState(false);
  const [successInvoice, setSuccessInvoice] =
    useState<SaleInvoiceSuccess | null>(null);
  const [whatsappPdfPath, setWhatsappPdfPath] = useState<string | null>(null);
  const [whatsappLoading, setWhatsappLoading] = useState(false);

  const debouncedSearch = useDebouncedValue(search, 150);

  const focusSearchInput = () => {
    searchInputRef.current?.focus();
    searchInputRef.current?.select();
  };

  if (!activeSession) {
    return (
      <div className="flex flex-col items-center justify-center h-full gap-6 text-center">
        <div className="text-7xl">🔒</div>
        <div className="space-y-2">
          <h2 className="text-2xl font-bold">لا توجد وردية مفتوحة</h2>
          <p className="text-muted-foreground text-lg">
            يجب فتح وردية للبدء في تسجيل المبيعات
          </p>
        </div>
        <Button
          size="lg"
          className="px-8"
          onClick={() => setOpenSessionDialogOpen(true)}
        >
          ⚡ بدء وردية جديدة
        </Button>
        <OpenSessionDialog
          open={openSessionDialogOpen}
          onOpenChange={setOpenSessionDialogOpen}
        />
      </div>
    );
  }

  const handlePrintInvoice = async () => {
    if (!successInvoice) {
      return;
    }

    try {
      const data = await getInvoicePrintDataWithLogo(successInvoice.id);
      printHtml(buildInvoicePrintHtml(data));
      toast.success("جاري الطباعة...");
    } catch (error) {
      toast.error(parseAppError(error).message_ar);
    }
  };

  const handleSendWhatsapp = async () => {
    if (!successInvoice) {
      return;
    }

    setWhatsappLoading(true);

    try {
      const pdfPath = await generateInvoicePdf(successInvoice.id);
      setWhatsappPdfPath(pdfPath);
      toast.success(`تم حفظ الفاتورة في: ${pdfPath}`);

      await openWhatsappWithInvoice({
        invoiceId: successInvoice.id,
        invoiceNumber: successInvoice.invoice_number,
      });

      toast.success("تم فتح واتساب، اختر جهة الاتصال ثم أرفق ملف PDF");
    } catch (error) {
      toast.error(parseAppError(error).message_ar);
    } finally {
      setWhatsappLoading(false);
    }
  };

  useEffect(() => {
    focusSearchInput();
  }, []);

  useEffect(() => {
    return () => {
      if (itemClickTimeoutRef.current !== null) {
        window.clearTimeout(itemClickTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    if (
      shouldSyncCashPaidToTotal({
        paymentMethod,
        paidCashMillieme,
        totalMillieme,
        paidCashManuallySet,
      })
    ) {
      setPaidCashAmount(totalMillieme);
    }
  }, [
    paidCashManuallySet,
    paidCashMillieme,
    paymentMethod,
    setPaidCashAmount,
    totalMillieme,
  ]);

  const categoriesQuery = usePosCategories();
  const itemsQuery = usePosItems(debouncedSearch, selectedCategoryId);
  const createSaleMutation = useCreateSaleInvoice();

  const items = itemsQuery.data ?? [];
  const categories = categoriesQuery.data ?? [];

  const deferredRemainingMillieme =
    paymentMethod === "deferred"
      ? Math.max(totalMillieme - paidCashMillieme - paidCardMillieme, 0)
      : 0;

  const isCartEmpty = cartItems.length === 0;
  const isSubmitDisabled = isCartEmpty || createSaleMutation.isPending;

  const handleAddItem = (item: Item) => {
    addItem(item);
    focusSearchInput();
  };

  const handleItemCardClick = (item: Item) => {
    if (itemClickTimeoutRef.current !== null) {
      window.clearTimeout(itemClickTimeoutRef.current);
    }

    itemClickTimeoutRef.current = window.setTimeout(() => {
      handleAddItem(item);
      itemClickTimeoutRef.current = null;
    }, 180);
  };

  const handleItemCardDoubleClick = (item: Item) => {
    if (itemClickTimeoutRef.current !== null) {
      window.clearTimeout(itemClickTimeoutRef.current);
      itemClickTimeoutRef.current = null;
    }

    setAdjustedItem(item);
    setIsAdjustDialogOpen(true);
  };

  const handleConfirmAdjustedItem = ({
    unitPriceMillieme,
    discountPercent,
  }: {
    unitPriceMillieme: number;
    discountPercent: number;
  }) => {
    if (!adjustedItem) {
      return;
    }

    addItem(adjustedItem, {
      unitPriceMillieme,
      discountPercent,
    });
    setAdjustedItem(null);
    focusSearchInput();
  };

  const handleClearInvoice = () => {
    if (!window.confirm("هل تريد مسح الفاتورة الحالية؟")) {
      return;
    }

    if (itemClickTimeoutRef.current !== null) {
      window.clearTimeout(itemClickTimeoutRef.current);
      itemClickTimeoutRef.current = null;
    }

    clearCart();
    setSearch("");
    setWhatsappLoading(false);
    setWhatsappPdfPath(null);
    focusSearchInput();
  };

  const handleNewInvoice = () => {
    if (itemClickTimeoutRef.current !== null) {
      window.clearTimeout(itemClickTimeoutRef.current);
      itemClickTimeoutRef.current = null;
    }

    clearCart();
    setSearch("");
    setSuccessInvoice(null);
    setWhatsappLoading(false);
    setWhatsappPdfPath(null);
    focusSearchInput();
  };

  const handleSubmitSale = async () => {
    if (!activeSession) {
      toast.error("لا توجد وردية مفتوحة");
      return;
    }

    if (paymentMethod === "cash" && paidCashMillieme < totalMillieme) {
      toast.error("المبلغ المدفوع أقل من إجمالي الفاتورة");
      return;
    }

    if (paymentMethod === "card" && paidCardMillieme !== totalMillieme) {
      toast.error("مبلغ الفيزا يجب أن يساوي إجمالي الفاتورة");
      return;
    }

    if (
      paymentMethod === "split" &&
      paidCashMillieme + paidCardMillieme !== totalMillieme
    ) {
      toast.error("يجب أن يساوي الدفع المختلط إجمالي الفاتورة");
      return;
    }

    const payload = buildSalePayload({
      sessionId: activeSession.id,
      customerId: null,
      customerName: customerName,
      customerPhone: customerPhone,
      paymentMethod,
      globalDiscountMillieme,
      paidCashMillieme,
      paidCardMillieme,
      notes,
      items: cartItems,
    });

    await createSaleMutation.mutateAsync(payload, {
      onSuccess: async (invoice) => {
        setSuccessInvoice(invoice);
        await invalidate(["pos-items"], ["items"], ["dashboard"]);
      },
      onError: (error) => {
        toast.error(parseAppError(error).message_ar);
      },
    });
  };

  return (
    <>
      <div className="min-h-[calc(100vh-81px)] p-4 lg:h-[calc(100vh-81px)] lg:overflow-hidden lg:p-6">
        <div className="flex h-full min-h-0 flex-col gap-4 lg:flex-row-reverse">
          <CatalogPanel
            searchInputRef={searchInputRef}
            search={search}
            onSearchChange={setSearch}
            categories={categories}
            selectedCategoryId={selectedCategoryId}
            onSelectCategory={setSelectedCategoryId}
            items={items}
            isLoading={itemsQuery.isLoading}
            onItemClick={handleItemCardClick}
            onItemDoubleClick={handleItemCardDoubleClick}
          />

          <Card className="flex min-h-[70vh] flex-col lg:h-full lg:min-h-0 lg:basis-[55%]">
            <CardContent className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4">
              <CartTable
                items={cartItems}
                onUpdateQty={updateQty}
                onUpdateLineDiscountPercent={updateLineDiscountPercent}
                onUpdateLineUnitPrice={updateLineUnitPrice}
                onRemoveItem={removeItem}
              />

              <TotalsPanel
                subtotalMillieme={subtotalMillieme}
                globalDiscountMillieme={globalDiscountMillieme}
                totalMillieme={totalMillieme}
                totalDiscountMillieme={totalDiscountMillieme}
                onSetGlobalDiscount={setGlobalDiscount}
              />

              <CustomerPanel
                customerName={customerName}
                customerPhone={customerPhone}
                onCustomerNameChange={setCustomerName}
                onCustomerPhoneChange={setCustomerPhone}
              />

              <PaymentPanel
                paymentMethod={paymentMethod}
                onSetPaymentMethod={setPaymentMethod}
                totalMillieme={totalMillieme}
                paidCashMillieme={paidCashMillieme}
                paidCardMillieme={paidCardMillieme}
                changeMillieme={changeMillieme}
                remainingMillieme={remainingMillieme}
                deferredRemainingMillieme={deferredRemainingMillieme}
                onSetPaidCashAmount={(amount) =>
                  setPaidCashAmount(amount, {
                    manual: amount !== totalMillieme,
                  })
                }
                onSetPaidCardAmount={setPaidCardAmount}
                onEnsureCashPaidAtLeastTotal={ensureCashPaidAtLeastTotal}
              />

              <InvoiceActionsPanel
                notes={notes}
                onNotesChange={setNotes}
                isSubmitDisabled={isSubmitDisabled}
                isSubmitting={createSaleMutation.isPending}
                totalLabel={formatEGP(totalMillieme)}
                onSubmit={() => void handleSubmitSale()}
                onClear={handleClearInvoice}
              />
            </CardContent>
          </Card>
        </div>
      </div>

      <InvoiceSuccessDialog
        open={Boolean(successInvoice)}
        invoice={successInvoice}
        onOpenChange={(open) => {
          if (!open) {
            setSuccessInvoice(null);
            setWhatsappLoading(false);
            setWhatsappPdfPath(null);
            focusSearchInput();
          }
        }}
        onPrint={() => void handlePrintInvoice()}
        onWhatsapp={() => void handleSendWhatsapp()}
        onNewInvoice={handleNewInvoice}
        whatsappLoading={whatsappLoading}
        pdfPath={whatsappPdfPath}
      />

      <InvoiceItemAdjustDialog
        open={isAdjustDialogOpen}
        item={adjustedItem}
        onOpenChange={(open) => {
          setIsAdjustDialogOpen(open);
          if (!open) {
            setAdjustedItem(null);
          }
        }}
        onConfirm={handleConfirmAdjustedItem}
      />
    </>
  );
}
