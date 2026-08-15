//! Read a Word document as Markdown.
//!
//! A `.docx` is a zip of OOXML parts; `word/document.xml` holds the body as a
//! flat run of paragraphs and tables. Headings and list membership live in each
//! paragraph's style, which is what the Markdown below is built from.

use std::path::PathBuf;

use docx_rs::{
    DocumentChild, Paragraph, ParagraphChild, RunChild, Table, TableCellContent, TableChild, TableRowChild,
};
use serde::{Deserialize, Serialize};

use crate::RunError;

#[derive(Debug, Serialize)]
pub struct Input {
    file_path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub text: String,
}

/// The visible text of a paragraph, with tabs and line breaks kept.
fn text(paragraph: &Paragraph) -> String {
    fn runs(children: &[ParagraphChild], into: &mut String) {
        for child in children {
            match child {
                ParagraphChild::Run(run) => {
                    for part in &run.children {
                        match part {
                            RunChild::Text(t) => into.push_str(&t.text),
                            RunChild::Tab(_) => into.push('\t'),
                            RunChild::Break(_) | RunChild::CarriageReturn(_) => into.push('\n'),
                            _ => {}
                        }
                    }
                }
                ParagraphChild::Hyperlink(link) => runs(&link.children, into),
                ParagraphChild::Insert(insert) => {
                    for child in &insert.children {
                        if let docx_rs::InsertChild::Run(run) = child {
                            for part in &run.children {
                                if let RunChild::Text(t) = part {
                                    into.push_str(&t.text);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = String::new();
    runs(&paragraph.children, &mut out);
    out
}

/// The heading level a paragraph's style names, if it names one.
fn level(paragraph: &Paragraph) -> Option<usize> {
    let style = paragraph.property.style.as_ref()?;
    let name = style.val.to_ascii_lowercase().replace(' ', "");
    let digits = name.strip_prefix("heading")?;
    digits.parse::<usize>().ok().filter(|n| (1..=6).contains(n))
}

fn listed(paragraph: &Paragraph) -> bool {
    paragraph.has_numbering || paragraph.property.numbering_property.is_some()
}

/// A table as GitHub-flavoured Markdown. The first row is the header, which is
/// what a Markdown table requires even when the document does not mark one.
fn table(table: &Table) -> String {
    let rows: Vec<Vec<String>> = table
        .rows
        .iter()
        .map(|TableChild::TableRow(row)| {
            row.cells
                .iter()
                .map(|TableRowChild::TableCell(cell)| {
                    cell.children
                        .iter()
                        .filter_map(|content| match content {
                            TableCellContent::Paragraph(paragraph) => Some(text(paragraph)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                        .replace('|', "\\|")
                        .trim()
                        .to_string()
                })
                .collect()
        })
        .collect();

    let Some(header) = rows.first() else {
        return String::new();
    };
    let mut out = vec![
        format!("| {} |", header.join(" | ")),
        format!("| {} |", vec!["---"; header.len()].join(" | ")),
    ];
    out.extend(rows[1..].iter().map(|row| format!("| {} |", row.join(" | "))));
    out.join("\n")
}

fn markdown(document: &[DocumentChild]) -> String {
    let mut blocks = Vec::new();
    for child in document {
        let block = match child {
            DocumentChild::Paragraph(paragraph) => {
                let body = text(paragraph);
                if body.trim().is_empty() {
                    continue;
                }
                match level(paragraph) {
                    Some(depth) => format!("{} {}", "#".repeat(depth), body.trim()),
                    None if listed(paragraph) => format!("- {}", body.trim()),
                    None => body.trim().to_string(),
                }
            }
            DocumentChild::Table(inner) => table(inner),
            _ => continue,
        };
        if !block.is_empty() {
            blocks.push(block);
        }
    }
    blocks.join("\n\n")
}

pub async fn parse_docx(file_path: PathBuf) -> Result<Output, RunError> {
    let text = tokio::task::spawn_blocking(move || {
        let bytes = std::fs::read(&file_path)
            .map_err(|e| RunError::CodeExecutionError(format!("could not read {}: {e}", file_path.display())))?;
        let document = docx_rs::read_docx(&bytes)
            .map_err(|e| RunError::CodeExecutionError(format!("{} is not a docx: {e}", file_path.display())))?;
        Ok::<_, RunError>(markdown(&document.document.children))
    })
    .await
    .map_err(|e| RunError::CodeExecutionError(format!("docx parsing did not finish: {e}")))??;

    Ok(Output { text })
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
    async fn test_parse_docx() {
        let parsed = parse_docx(file("decision_log.docx")).await.unwrap();
        println!("parsed_docx: {:?}", parsed);
        assert!(parsed.text.contains("Approved backend languages are Go, Python"));
    }

    #[tokio::test]
    async fn headings_and_paragraphs_come_back_as_markdown() {
        let parsed = parse_docx(file("decision_log.docx")).await.unwrap();
        assert!(!parsed.text.trim().is_empty());
        // Blocks are separated, never run together.
        assert!(parsed.text.contains("\n\n"));
        // No stray XML survived the read.
        assert!(!parsed.text.contains("<w:"), "raw OOXML leaked into the text");
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_docx_is_an_error() {
        assert!(parse_docx(PathBuf::from("/nonexistent/nope.docx")).await.is_err());
        assert!(parse_docx(file("hanzo_intro.pdf")).await.is_err());
    }
}
