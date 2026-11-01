import { Phone, UserRound } from "lucide-react";

import { Input } from "@/components/ui/input";

export function CustomerPanel({
  customerName,
  customerPhone,
  onCustomerNameChange,
  onCustomerPhoneChange,
}: {
  customerName: string;
  customerPhone: string;
  onCustomerNameChange: (value: string) => void;
  onCustomerPhoneChange: (value: string) => void;
}) {
  return (
    <div className="space-y-3 rounded-2xl border p-4">
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="space-y-2">
          <label className="text-sm font-medium">اسم العميل</label>
          <div className="relative">
            <UserRound className="absolute inset-e-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              dir="rtl"
              className="pe-9"
              placeholder="اسم العميل (اختياري)"
              value={customerName}
              onChange={(e) => onCustomerNameChange(e.target.value)}
            />
          </div>
        </div>
        <div className="space-y-2">
          <label className="text-sm font-medium">رقم الهاتف</label>
          <div className="relative">
            <Phone className="absolute inset-e-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              dir="ltr"
              className="pe-9 text-right"
              placeholder="رقم الهاتف (اختياري)"
              value={customerPhone}
              onChange={(e) => onCustomerPhoneChange(e.target.value)}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
