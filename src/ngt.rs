// The .ngt text format:
//
//   !start name           (optional; defaults to a category called "name")
//
//   first = Anna, Beth:2, Clara
//   last = Smith, Jones
//   name = {first} {last}  # comment
//
// Each category line is `name = entry, entry, ...`. An entry may end in
// `:<weight>` (a positive integer, default 1) and may reference another
// category with `{other_name}`. Lines starting with `#` and blank lines are
// ignored, and a `#` anywhere else on a line starts a comment that runs to
// the end of the line; write `\#` for a literal `#` in an entry.
//
// A colon inside an entry's text is only treated as a weight separator when
// everything after it is digits, so ordinary punctuation is left alone.
//
// `\{` and `\}` in an entry escape a brace so it reads as literal text
// instead of a `{placeholder}` reference. This is a property of the shared
// entry text, not just .ngt syntax, so it round-trips through .ngj unchanged
// (see model::placeholders and sample::render).

use crate::model::{Document, Entry, Issue};

pub fn parse(input: &str) -> Result<(Document, Vec<Issue>), Issue> {
    let mut doc = Document::default();
    let mut issues = Vec::new();

    for (line_no, raw_line) in input.lines().enumerate() {
        let line_no = line_no + 1;
        let stripped = strip_comment(raw_line);
        let line = stripped.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("!start") {
            let name = rest.trim();
            if name.is_empty() {
                return Err(Issue::fatal(format!(
                    "line {}: !start requires a category name",
                    line_no
                )));
            }
            if doc.start.is_some() {
                issues.push(Issue::semantic(format!(
                    "line {}: duplicate !start directive; using the last one",
                    line_no
                )));
            }
            doc.start = Some(name.to_string());
            continue;
        }

        let eq = line
            .find('=')
            .ok_or_else(|| Issue::fatal(format!("line {}: expected '<name> = <entries>'", line_no)))?;
        let name = line[..eq].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(Issue::fatal(format!(
                "line {}: '{}' is not a valid category name",
                line_no, name
            )));
        }

        let rest = line[eq + 1..].trim();
        if rest.is_empty() {
            issues.push(Issue::semantic(format!(
                "line {}: category '{}' has no entries",
                line_no, name
            )));
        }

        let mut entries = Vec::new();
        for raw_entry in rest.split(',') {
            let entry = raw_entry.trim();
            if entry.is_empty() {
                continue;
            }
            entries.push(parse_entry(entry));
        }

        if doc.add_category(name.to_string(), entries) {
            issues.push(Issue::semantic(format!(
                "line {}: duplicate category '{}'; entries were merged",
                line_no, name
            )));
        }
    }

    Ok((doc, issues))
}

// Cuts a line at the first unescaped `#` and unescapes `\#` into a literal
// `#` in what's kept. Other backslash sequences (like `\{`) are left alone
// here; they're part of the shared entry-text escape handled downstream.
fn strip_comment(line: &str) -> String {
    let mut result = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'#') {
            result.push('#');
            chars.next();
            continue;
        }
        if c == '#' {
            break;
        }
        result.push(c);
    }
    result
}

fn parse_entry(raw: &str) -> Entry {
    if let Some(idx) = raw.rfind(':') {
        let text_part = &raw[..idx];
        let weight_part = &raw[idx + 1..];
        if !text_part.is_empty() && !weight_part.is_empty() && weight_part.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(weight) = weight_part.parse::<u32>() {
                if weight >= 1 {
                    return Entry { text: text_part.trim().to_string(), weight };
                }
            }
        }
    }
    Entry { text: raw.trim().to_string(), weight: 1 }
}

pub fn write(doc: &Document) -> String {
    let mut out = String::new();
    if let Some(start) = &doc.start {
        out.push_str("!start ");
        out.push_str(start);
        out.push_str("\n\n");
    }
    for (name, entries) in &doc.categories {
        out.push_str(name);
        out.push_str(" = ");
        let rendered: Vec<String> = entries
            .iter()
            .map(|e| {
                let text = e.text.replace('#', "\\#");
                if e.weight == 1 {
                    text
                } else {
                    format!("{}:{}", text, e.weight)
                }
            })
            .collect();
        out.push_str(&rendered.join(", "));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailing_comment_is_stripped() {
        let (doc, issues) = parse("first = Anna, Beth  # placeholder names\n").unwrap();
        assert!(issues.is_empty());
        assert_eq!(doc.categories, vec![(
            "first".to_string(),
            vec![Entry { text: "Anna".to_string(), weight: 1 }, Entry { text: "Beth".to_string(), weight: 1 }],
        )]);
    }

    #[test]
    fn full_line_comment_is_ignored() {
        let (doc, issues) = parse("# just a note\nfirst = Anna\n").unwrap();
        assert!(issues.is_empty());
        assert_eq!(doc.categories.len(), 1);
    }

    #[test]
    fn escaped_hash_is_kept_literal() {
        let (doc, issues) = parse("first = C\\# is a language # comment\n").unwrap();
        assert!(issues.is_empty());
        assert_eq!(doc.categories[0].1[0].text, "C# is a language");
    }

    #[test]
    fn escaped_braces_pass_through_unchanged() {
        let (doc, issues) = parse("name = \\{literal\\}\n").unwrap();
        assert!(issues.is_empty());
        assert_eq!(doc.categories[0].1[0].text, "\\{literal\\}");
    }

    #[test]
    fn literal_hash_round_trips_through_write() {
        let (doc, _) = parse("first = C\\#\n").unwrap();
        let written = write(&doc);
        let (reparsed, issues) = parse(&written).unwrap();
        assert!(issues.is_empty());
        assert_eq!(doc, reparsed);
    }
}
