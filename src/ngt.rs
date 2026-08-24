// The .ngt text format:
//
//   !start name           (optional; defaults to a category called "name")
//
//   first = Anna, Beth:2, Clara
//   last = Smith, Jones
//   name = {first} {last}
//
// Each category line is `name = entry, entry, ...`. An entry may end in
// `:<weight>` (a positive integer, default 1) and may reference another
// category with `{other_name}`. Lines starting with `#` and blank lines are
// ignored.
//
// A colon inside an entry's text is only treated as a weight separator when
// everything after it is digits, so ordinary punctuation is left alone.

use crate::model::{Document, Entry, Issue};

pub fn parse(input: &str) -> Result<(Document, Vec<Issue>), Issue> {
    let mut doc = Document::default();
    let mut issues = Vec::new();

    for (line_no, raw_line) in input.lines().enumerate() {
        let line_no = line_no + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
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
                if e.weight == 1 {
                    e.text.clone()
                } else {
                    format!("{}:{}", e.text, e.weight)
                }
            })
            .collect();
        out.push_str(&rendered.join(", "));
        out.push('\n');
    }
    out
}
