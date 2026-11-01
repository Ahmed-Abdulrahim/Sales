
/// Item with its supplier information, for the purchases search.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct ItemWithSupplier {
    pub id: i64,
    pub name_ar: String,
    pub buy_price_millieme: i64,
    pub sell_price_millieme: i64,
    pub current_stock: i64,
    pub unit: String,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
}
