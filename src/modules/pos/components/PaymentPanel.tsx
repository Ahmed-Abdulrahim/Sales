import { Input } from "@/components/ui/input";
import { PaymentTab } from "@/modules/pos/components/PosControls";
import type { PaymentMethod } from "@/modules/pos/types";
import { moneyToInput } from "@/modules/pos/utils";
import { formatEGP, toMillieme } from "@/shared/utils/money";

export function PaymentPanel({
  paymentMethod,
  onSetPaymentMethod,
  totalMillieme,
  paidCashMillieme,
  paidCardMillieme,
  changeMillieme,
  remainingMillieme,
  deferredRemainingMillieme,
  onSetPaidCashAmount,
  onSetPaidCardAmount,
  onEnsureCashPaidAtLeastTotal,
}: {
  paymentMethod: PaymentMethod;
  onSetPaymentMethod: (method: PaymentMethod) => void;
  totalMillieme: number;
  paidCashMillieme: number;
  paidCardMillieme: number;
  changeMillieme: number;
  remainingMillieme: number;
  deferredRemainingMillieme: number;
  onSetPaidCashAmount: (amount: number) => void;
  onSetPaidCardAmount: (amount: number) => void;
  onEnsureCashPaidAtLeastTotal: (
    paidMillieme: number,
    totalMillieme: number,
  ) => number;
}) {
  const paymentMethodLabel: Record<PaymentMethod, string> = {
    cash: "كاش",
    card: "فيزا",
    deferred: "آجل",
    split: "دفع مختلط",
  };

  return (
    <div className="space-y-3 rounded-2xl border p-4">
      <div className="flex flex-wrap gap-2">
        {(Object.entries(paymentMethodLabel) as Array<[PaymentMethod, string]>).map(
          ([value, label]) => (
            <PaymentTab
              key={value}
              active={paymentMethod === value}
              onClick={() => onSetPaymentMethod(value)}
            >
              {label}
            </PaymentTab>
          ),
        )}
      </div>

      {paymentMethod === "cash" ? (
        <div className="space-y-2">
          <label className="text-sm font-medium">المبلغ المدفوع</label>
          <Input
            dir="rtl"
            type="number"
            min={moneyToInput(totalMillieme)}
            step="0.01"
            value={moneyToInput(paidCashMillieme)}
            onChange={(event) => {
              try {
                const parsedMillieme = toMillieme(event.target.value || 0);
                onSetPaidCashAmount(
                  onEnsureCashPaidAtLeastTotal(parsedMillieme, totalMillieme),
                );
              } catch {
                onSetPaidCashAmount(totalMillieme);
              }
            }}
            onBlur={() => {
              if (paidCashMillieme < totalMillieme) {
                onSetPaidCashAmount(
                  onEnsureCashPaidAtLeastTotal(paidCashMillieme, totalMillieme),
                );
              }
            }}
          />
          <p className="text-sm text-muted-foreground">
            الباقي: {formatEGP(Math.max(changeMillieme, 0))}
          </p>
        </div>
      ) : null}

      {paymentMethod === "card" ? (
        <div className="space-y-2">
          <label className="text-sm font-medium">مبلغ الفيزا</label>
          <Input
            dir="rtl"
            type="number"
            min={0}
            step="0.01"
            value={moneyToInput(paidCardMillieme)}
            onChange={(event) => {
              try {
                onSetPaidCardAmount(toMillieme(event.target.value || 0));
              } catch {
                onSetPaidCardAmount(0);
              }
            }}
          />
        </div>
      ) : null}

      {paymentMethod === "deferred" ? (
        <div className="space-y-2">
          <label className="text-sm font-medium">المدفوع الآن</label>
          <Input
            dir="rtl"
            type="number"
            min={0}
            step="0.01"
            value={moneyToInput(paidCashMillieme)}
            onChange={(event) => {
              try {
                onSetPaidCashAmount(toMillieme(event.target.value || 0));
                onSetPaidCardAmount(0);
              } catch {
                onSetPaidCashAmount(0);
                onSetPaidCardAmount(0);
              }
            }}
          />
          <p className="text-sm text-muted-foreground">
            المتبقي على الحساب: {formatEGP(deferredRemainingMillieme)}
          </p>
        </div>
      ) : null}

      {paymentMethod === "split" ? (
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="space-y-2">
            <label className="text-sm font-medium">نقدي</label>
            <Input
              dir="rtl"
              type="number"
              min={0}
              step="0.01"
              value={moneyToInput(paidCashMillieme)}
              onChange={(event) => {
                try {
                  onSetPaidCashAmount(toMillieme(event.target.value || 0));
                } catch {
                  onSetPaidCashAmount(0);
                }
              }}
            />
          </div>
          <div className="space-y-2">
            <label className="text-sm font-medium">فيزا</label>
            <Input
              dir="rtl"
              type="number"
              min={0}
              step="0.01"
              value={moneyToInput(paidCardMillieme)}
              onChange={(event) => {
                try {
                  onSetPaidCardAmount(toMillieme(event.target.value || 0));
                } catch {
                  onSetPaidCardAmount(0);
                }
              }}
            />
          </div>
          <p className="text-sm text-muted-foreground sm:col-span-2">
            المدفوع حاليًا: {formatEGP(paidCashMillieme + paidCardMillieme)}
          </p>
          {remainingMillieme > 0 ? (
            <p className="text-sm font-medium text-amber-700 sm:col-span-2">
              المتبقي: {formatEGP(remainingMillieme)}
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
