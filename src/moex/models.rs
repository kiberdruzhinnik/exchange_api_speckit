use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstrumentCategory {
    Shares,
    Index,
    Currency,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoardAssignment {
    pub engine: String,
    pub market: String,
    pub board: String,
    pub primary: bool,
    pub history_from: String,
    pub history_till: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedInstrument {
    pub category: InstrumentCategory,
    pub engine: String,
    pub market: String,
    pub boards: Vec<BoardAssignment>,
    pub lotsizes: std::collections::HashMap<String, Option<f64>>,
}

impl ResolvedInstrument {
    pub fn current_board(&self) -> Option<&BoardAssignment> {
        self.boards.iter().find(|board| board.primary)
    }
}

#[derive(Debug)]
pub struct IssTable {
    pub columns: Vec<String>,
    pub data: Vec<Vec<Value>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LatestTrade {
    pub trade_date: String,
    pub trade_time: String,
    pub price: f64,
    pub quantity: f64,
}

pub fn parse_latest_trade(value: &Value) -> anyhow::Result<Option<LatestTrade>> {
    let table = IssTable::parse(&value["trades"])?;
    let date = table.index("TRADEDATE")?;
    let time = table.index("TRADETIME")?;
    let price = table.index("PRICE")?;
    let quantity = table.index("QUANTITY")?;
    let Some(row) = table.data.first() else {
        return Ok(None);
    };
    let string = |index: usize, name: &str| {
        row.get(index)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow::anyhow!("latest trade missing {name}"))
    };
    let number = |index: usize, name: &str| {
        row.get(index)
            .and_then(|v| v.as_f64().or_else(|| v.as_str()?.parse().ok()))
            .filter(|n| n.is_finite())
            .ok_or_else(|| anyhow::anyhow!("latest trade has invalid {name}"))
    };
    Ok(Some(LatestTrade {
        trade_date: string(date, "TRADEDATE")?,
        trade_time: string(time, "TRADETIME")?,
        price: number(price, "PRICE")?,
        quantity: number(quantity, "QUANTITY")?,
    }))
}

impl IssTable {
    pub fn parse(value: &Value) -> anyhow::Result<Self> {
        let columns = value
            .get("columns")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("ISS table missing columns"))?
            .iter()
            .map(|v| {
                v.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| anyhow::anyhow!("invalid ISS column"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let data = value
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("ISS table missing data"))?
            .iter()
            .map(|row| {
                row.as_array()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("invalid ISS row"))
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(Self { columns, data })
    }
    pub fn index(&self, name: &str) -> anyhow::Result<usize> {
        self.columns
            .iter()
            .position(|column| column.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow::anyhow!("ISS table missing {name}"))
    }
}
