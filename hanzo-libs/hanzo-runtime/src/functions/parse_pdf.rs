//! Extract a PDF's text, one entry per page.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::RunError;

#[derive(serde::Serialize)]
pub struct Input {
    file_path: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    pub pages: Vec<Page>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Page {
    pub metadata: Metadata,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    /// Page number, counting from one.
    pub page: u32,
}

pub async fn parse_pdf(file_path: PathBuf) -> Result<Output, RunError> {
    let pages = tokio::task::spawn_blocking(move || {
        pdf_extract::extract_text_by_pages(&file_path)
            .map_err(|e| RunError::CodeExecutionError(format!("could not read {}: {e}", file_path.display())))
    })
    .await
    .map_err(|e| RunError::CodeExecutionError(format!("pdf parsing did not finish: {e}")))??;

    Ok(Output {
        pages: pages
            .into_iter()
            .enumerate()
            .map(|(index, text)| Page {
                metadata: Metadata { page: index as u32 + 1 },
                text,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path;
    use std::path::Path;

    fn file(name: &str) -> PathBuf {
        path::absolute(Path::new("../../files").join(name)).unwrap()
    }

    #[tokio::test]
    async fn test_parse_pdf_file() {
        let parsed = parse_pdf(file("Hanzo_Protocol_Whitepaper.pdf")).await.unwrap();
        assert_eq!(parsed.pages.len(), 14);

        // Pages are numbered from one, in order.
        for (index, page) in parsed.pages.iter().enumerate() {
            assert_eq!(page.metadata.page, index as u32 + 1);
        }

        // The last page carries the MAC computation note.
        let last = &parsed.pages[13].text;
        assert!(last.contains("Essential for MAC"), "last page reads: {last}");
        assert!(
            parsed.pages.iter().all(|page| !page.text.trim().is_empty()),
            "every page of this document has text"
        );
    }

    #[tokio::test]
    async fn every_page_of_a_short_document_is_returned() {
        let parsed = parse_pdf(file("hanzo_intro.pdf")).await.unwrap();
        assert!(!parsed.pages.is_empty());
        assert!(
            parsed.pages.iter().any(|page| !page.text.trim().is_empty()),
            "expected text on at least one page"
        );
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_pdf_is_an_error() {
        assert!(parse_pdf(PathBuf::from("/nonexistent/nope.pdf")).await.is_err());
        assert!(parse_pdf(file("decision_log.docx")).await.is_err());
    }
}
