use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "Valuta")]
pub struct CurrencyDirectory {
    #[serde(rename = "Item", default)]
    pub items: Vec<CurrencyItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CurrencyItem {
    #[serde(rename = "@ID")]
    pub id: String,
    #[serde(rename = "ISO_Char_Code")]
    pub char_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "ValCurs")]
pub struct DailyRates {
    #[serde(rename = "@Date")]
    pub date: Option<String>,
    #[serde(rename = "Valute", default)]
    pub rates: Vec<DailyRate>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DailyRate {
    #[serde(rename = "CharCode")]
    pub char_code: Option<String>,
    #[serde(rename = "Nominal")]
    pub nominal: Option<String>,
    #[serde(rename = "Value")]
    pub value: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename = "ValCurs")]
pub struct DynamicRates {
    #[serde(rename = "Record", default)]
    pub records: Vec<DynamicRate>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct DynamicRate {
    #[serde(rename = "@Date")]
    pub date: Option<String>,
    #[serde(rename = "Nominal")]
    pub nominal: Option<String>,
    #[serde(rename = "Value")]
    pub value: Option<String>,
}
