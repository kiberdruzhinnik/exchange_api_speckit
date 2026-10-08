use serde_json::Value;

#[derive(Debug)]
pub struct IssTable {
    pub columns: Vec<String>,
    pub data: Vec<Vec<Value>>,
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
