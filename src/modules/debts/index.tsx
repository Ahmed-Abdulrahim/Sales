import { useQuery } from "@tanstack/react-query";
import { AlertCircle } from "lucide-react";

import { invoke } from "@/shared/utils/invoke";
import type { InvoiceSummary } from "@/modules/sales/types";
import { formatEGP } from "@/shared/utils/money";
import { SectionCard } from "@/shared/components/SectionCard";

export default function DebtsPage() {
  const debtsQuery = useQuery({
    queryKey: ["debts"],
    queryFn: () => invoke<InvoiceSummary[]>("list_debts"),
    refetchOnMount: "always",
    refetchOnWindowFocus: true,
  });

  const debts = debtsQuery.data ?? [];
  const totalDebt = debts.reduce(
    (sum, inv) => sum + inv.total_millieme - inv.paid_millieme,
    0,
  );

  return (
    <div className="space-y-6 p-6">
      <header className="space-y-1">
        <h1 className="text-3xl font-semibold tracking-tight">مديونيات العملاء</h1>
        <p className="text-sm text-muted-foreground">
          الفواتير الآجلة والجزئية التي لم يتم سدادها بالكامل
        </p>
      </header>

      {debtsQuery.isLoading ? (
        <div className="text-center py-12 text-muted-foreground">جاري التحميل...</div>
      ) : debtsQuery.isError ? (
        <div className="text-center py-12 text-destructive">حدث خطأ أثناء تحميل البيانات</div>
      ) : debts.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-24 gap-4 text-muted-foreground">
          <AlertCircle className="size-12 opacity-40" />
          <p className="text-lg">لا توجد مديونيات حالياً</p>
        </div>
      ) : (
        <SectionCard title={`إجمالي المديونيات: ${formatEGP(totalDebt)}`}>
          <div className="overflow-x-auto">
            <table className="w-full text-sm text-right" dir="rtl">
              <thead>
                <tr className="border-b text-muted-foreground">
                  <th className="py-3 px-4 font-medium">رقم الفاتورة</th>
                  <th className="py-3 px-4 font-medium">العميل</th>
                  <th className="py-3 px-4 font-medium">الإجمالي</th>
                  <th className="py-3 px-4 font-medium">المدفوع</th>
                  <th className="py-3 px-4 font-medium">المتبقي</th>
                  <th className="py-3 px-4 font-medium">الحالة</th>
                  <th className="py-3 px-4 font-medium">التاريخ</th>
                </tr>
              </thead>
              <tbody>
                {debts.map((inv) => {
                  const remaining = inv.total_millieme - inv.paid_millieme;
                  return (
                    <tr key={inv.id} className="border-b hover:bg-muted/30 transition-colors">
                      <td className="py-3 px-4 font-mono font-semibold">{inv.invoice_number}</td>
                      <td className="py-3 px-4">{inv.customer_name ?? "—"}</td>
                      <td className="py-3 px-4">{formatEGP(inv.total_millieme)}</td>
                      <td className="py-3 px-4 text-emerald-700">{formatEGP(inv.paid_millieme)}</td>
                      <td className="py-3 px-4 text-red-700 font-semibold">{formatEGP(remaining)}</td>
                      <td className="py-3 px-4">
                        <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${
                          inv.status === "partial"
                            ? "bg-amber-100 text-amber-800"
                            : "bg-red-100 text-red-800"
                        }`}>
                          {inv.status === "partial" ? "جزئي" : "آجل"}
                        </span>
                      </td>
                      <td className="py-3 px-4 text-muted-foreground">
                        {inv.created_at.slice(0, 10)}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </SectionCard>
      )}
    </div>
  );
}
