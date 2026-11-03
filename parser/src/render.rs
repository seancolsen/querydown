//! Renders an [`Error`] as a multi-line diagnostic that quotes the relevant source lines.

use std::fmt::Write;

use crate::error::{Error, Span};

/// One underlined region to draw beneath a source line.
struct Annotation<'a> {
    span: Span,
    /// `^` for the primary span, `-` for secondary labels.
    marker: char,
    message: Option<&'a str>,
}

pub(crate) fn render(error: &Error, source: &str) -> String {
    let mut out = format!("{}: {}", error.kind, error.message);

    let Some(primary) = error.span else {
        if let Some(hint) = &error.hint {
            write!(out, "\nhint: {hint}").unwrap();
        }
        return out;
    };

    let mut annotations = vec![Annotation {
        span: primary,
        marker: '^',
        message: None,
    }];
    annotations.extend(error.labels.iter().map(|label| Annotation {
        span: label.span,
        marker: '-',
        message: Some(label.message.as_str()),
    }));

    let lines: Vec<&str> = source.split('\n').collect();
    let mut line_numbers: Vec<usize> = annotations.iter().map(|a| a.span.start.line).collect();
    line_numbers.sort_unstable();
    line_numbers.dedup();
    let gutter = line_numbers.last().map_or(1, |n| n.to_string().len());
    let pad = " ".repeat(gutter);

    write!(
        out,
        "\n{pad}--> line {}, column {}\n{pad} |",
        primary.start.line, primary.start.column
    )
    .unwrap();

    let mut previous_line: Option<usize> = None;
    for line_number in line_numbers {
        if matches!(previous_line, Some(prev) if line_number > prev + 1) {
            write!(out, "\n{pad} ...").unwrap();
        }
        previous_line = Some(line_number);

        let text = lines.get(line_number - 1).copied().unwrap_or("");
        let text = text.strip_suffix('\r').unwrap_or(text);
        // Tabs are shown as single spaces so that the underlines, which are positioned by `char`
        // count, line up with the text above them.
        let shown: String = text
            .chars()
            .map(|c| if c == '\t' { ' ' } else { c })
            .collect();
        write!(out, "\n{line_number:>gutter$} | {shown}").unwrap();

        let line_len = text.chars().count();
        for annotation in annotations
            .iter()
            .filter(|a| a.span.start.line == line_number)
        {
            let start_col = annotation.span.start.column;
            let end_col = if annotation.span.end.line == line_number {
                annotation.span.end.column
            } else {
                // A multi-line span is underlined to the end of its first line.
                line_len + 1
            };
            let width = end_col.saturating_sub(start_col).max(1);
            let mut underline = format!(
                "{}{}",
                " ".repeat(start_col - 1),
                annotation.marker.to_string().repeat(width)
            );
            if let Some(message) = annotation.message {
                write!(underline, " {message}").unwrap();
            }
            write!(out, "\n{pad} | {underline}").unwrap();
        }
    }

    if let Some(hint) = &error.hint {
        write!(out, "\n{pad} |\n{pad} = hint: {hint}").unwrap();
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::error::{Error, ErrorKind, Span};

    #[test]
    fn test_render_without_span() {
        let err = Error::compile("Column `x` not found.").with_hint("Check the spelling.");
        assert_eq!(
            err.render("#t x:1"),
            "compile error: Column `x` not found.\nhint: Check the spelling."
        );
    }

    #[test]
    fn test_render_with_span_and_label() {
        let source = "#t\nfoo:[1 2\n  )";
        let err = Error::new(ErrorKind::Syntax, "Mismatched delimiter.")
            .with_span(Span::from_range(source, 14..15))
            .with_label(Span::from_range(source, 7..8), "`[` opened here")
            .with_hint("Replace `)` with `]`.");
        assert_eq!(
            err.render(source),
            [
                "syntax error: Mismatched delimiter.",
                " --> line 3, column 3",
                "  |",
                "2 | foo:[1 2",
                "  |     - `[` opened here",
                "3 |   )",
                "  |   ^",
                "  |",
                "  = hint: Replace `)` with `]`.",
            ]
            .join("\n")
        );
    }

    #[test]
    fn test_render_elides_gaps_and_handles_empty_spans() {
        let source = "(\na\nb\n";
        let err = Error::new(ErrorKind::Syntax, "Unclosed.")
            .with_span(Span::from_range(source, 6..6))
            .with_label(Span::from_range(source, 0..1), "opened here");
        assert_eq!(
            err.render(source),
            [
                "syntax error: Unclosed.",
                " --> line 4, column 1",
                "  |",
                "1 | (",
                "  | - opened here",
                "  ...",
                "4 | ",
                "  | ^",
            ]
            .join("\n")
        );
    }
}
