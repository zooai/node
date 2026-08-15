//! Read the first worksheet of a spreadsheet as rows of cells.
//!
//! Covers both the OOXML workbook (`.xlsx`) and the older BIFF one (`.xls`).

use std::path::PathBuf;

use calamine::{open_workbook_auto, Data, Reader};
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::RunError;

#[derive(Debug, Serialize)]
pub struct Input {
    file_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub rows: Vec<Vec<Value>>,
}

/// A cell as JSON. Blank and errored cells are null so that every row keeps
/// its column positions.
fn cell(data: &Data) -> Value {
    let number = |value: f64| Number::from_f64(value).map(Value::Number).unwrap_or(Value::Null);
    match data {
        Data::Empty => Value::Null,
        Data::String(text) => Value::String(text.clone()),
        Data::Int(value) => Value::Number((*value).into()),
        Data::Float(value) => number(*value),
        Data::Bool(value) => Value::Bool(*value),
        // Dates keep their serial number, which is what the sheet stores.
        Data::DateTime(value) => number(value.as_f64()),
        Data::DateTimeIso(text) | Data::DurationIso(text) => Value::String(text.clone()),
        Data::Error(_) => Value::Null,
    }
}

pub async fn parse_xlsx(file_path: PathBuf) -> Result<Output, RunError> {
    let rows = tokio::task::spawn_blocking(move || {
        let mut workbook = open_workbook_auto(&file_path)
            .map_err(|e| RunError::CodeExecutionError(format!("could not open {}: {e}", file_path.display())))?;
        let sheet = workbook
            .sheet_names()
            .first()
            .cloned()
            .ok_or_else(|| RunError::CodeExecutionError(format!("{} has no worksheets", file_path.display())))?;
        let range = workbook
            .worksheet_range(&sheet)
            .map_err(|e| RunError::CodeExecutionError(format!("could not read sheet {sheet}: {e}")))?;
        Ok::<_, RunError>(range.rows().map(|row| row.iter().map(cell).collect()).collect())
    })
    .await
    .map_err(|e| RunError::CodeExecutionError(format!("spreadsheet parsing did not finish: {e}")))??;

    Ok(Output { rows })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path;
    use std::path::Path;

    #[tokio::test]
    async fn test_parse_xlsx() {
        let xlsx_file_path = path::absolute(Path::new("../hanzo-fs/src/test_data/test.xlsx"))
            .unwrap()
            .to_path_buf();
        let rows = parse_xlsx(xlsx_file_path).await.unwrap();
        assert_eq!(rows.rows.len(), 4);
        assert_eq!(rows.rows[0].len(), 2);
    }

    /// The sheet holds ten populated rows. Its declared extent runs one row
    /// further, so a reader that trusts the extent rather than the content
    /// reports an eleventh row of nothing; this one reports what is there.
    #[tokio::test]
    async fn test_parse_xls() {
        let xlsx_file_path = path::absolute(Path::new("../hanzo-fs/src/test_data/test.xls"))
            .unwrap()
            .to_path_buf();
        let rows = parse_xlsx(xlsx_file_path).await.unwrap();
        assert_eq!(rows.rows.len(), 10);
        assert_eq!(rows.rows[0].len(), 8);
        assert_eq!(rows.rows[0][1], "First Name");
        assert_eq!(rows.rows[8][1], "Earlean");
        assert!(rows.rows.iter().all(|row| row.iter().any(|cell| !cell.is_null())));
    }

    #[tokio::test]
    async fn cells_keep_their_json_type() {
        let xlsx_file_path = path::absolute(Path::new("../hanzo-fs/src/test_data/test.xlsx"))
            .unwrap()
            .to_path_buf();
        let rows = parse_xlsx(xlsx_file_path).await.unwrap().rows;
        // Every row is rectangular, and cells are strings, numbers, bools or null.
        let width = rows[0].len();
        for row in &rows {
            assert_eq!(row.len(), width);
            for value in row {
                assert!(
                    value.is_string() || value.is_number() || value.is_boolean() || value.is_null(),
                    "unexpected cell {value:?}"
                );
            }
        }
        assert!(
            rows.iter().flatten().any(|value| value.is_string()),
            "expected some text in the sheet"
        );
    }

    #[tokio::test]
    async fn a_missing_file_is_an_error() {
        assert!(parse_xlsx(PathBuf::from("/nonexistent/nope.xlsx")).await.is_err());
    }
}
