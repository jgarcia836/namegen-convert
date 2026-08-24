// The .ngj format is the same document model as JSON:
//
//   {
//     "start": "name",
//     "categories": {
//       "first": ["Anna", {"text": "Beth", "weight": 2}, "Clara"],
//       "last": ["Smith", "Jones"],
//       "name": ["{first} {last}"]
//     }
//   }
//
// Entries can be a bare string (weight 1) or an object with "text" and an
// optional "weight".

use crate::json::{self, Value};
use crate::model::{Document, Entry, Issue};

pub fn parse(input: &str) -> Result<(Document, Vec<Issue>), Issue> {
    let value = json::parse(input).map_err(Issue::fatal)?;
    let obj = value
        .as_object()
        .ok_or_else(|| Issue::fatal("top-level JSON value must be an object".to_string()))?;

    let mut doc = Document::default();
    let mut issues = Vec::new();

    for (key, val) in obj {
        match key.as_str() {
            "start" => match val.as_str() {
                Some(s) => doc.start = Some(s.to_string()),
                None => issues.push(Issue::semantic("\"start\" must be a string; ignoring it".to_string())),
            },
            "categories" => {
                let cats = val
                    .as_object()
                    .ok_or_else(|| Issue::fatal("\"categories\" must be an object".to_string()))?;
                for (name, entries_val) in cats {
                    let entries = parse_entries(entries_val, name, &mut issues)?;
                    if doc.add_category(name.clone(), entries) {
                        issues.push(Issue::semantic(format!(
                            "duplicate category '{}'; entries were merged",
                            name
                        )));
                    }
                }
            }
            other => issues.push(Issue::semantic(format!("unknown top-level key '{}' was ignored", other))),
        }
    }

    Ok((doc, issues))
}

fn parse_entries(val: &Value, category: &str, issues: &mut Vec<Issue>) -> Result<Vec<Entry>, Issue> {
    let items = val
        .as_array()
        .ok_or_else(|| Issue::fatal(format!("category '{}' must be an array of entries", category)))?;

    let mut entries = Vec::new();
    for item in items {
        match item {
            Value::String(s) => entries.push(Entry { text: s.clone(), weight: 1 }),
            Value::Object(fields) => {
                let mut text = None;
                let mut weight = 1u32;
                for (field, field_val) in fields {
                    match field.as_str() {
                        "text" => text = field_val.as_str().map(|s| s.to_string()),
                        "weight" => match field_val.as_f64() {
                            Some(n) if n >= 1.0 && n.fract() == 0.0 => weight = n as u32,
                            _ => issues.push(Issue::semantic(format!(
                                "category '{}' has an invalid weight; defaulting to 1",
                                category
                            ))),
                        },
                        other => issues.push(Issue::semantic(format!(
                            "category '{}' entry has unknown field '{}'",
                            category, other
                        ))),
                    }
                }
                match text {
                    Some(t) => entries.push(Entry { text: t, weight }),
                    None => issues.push(Issue::semantic(format!(
                        "category '{}' has an entry object without a \"text\" field; skipping it",
                        category
                    ))),
                }
            }
            _ => issues.push(Issue::semantic(format!(
                "category '{}' has an entry that is neither a string nor an object; skipping it",
                category
            ))),
        }
    }
    Ok(entries)
}

pub fn write(doc: &Document) -> String {
    let mut cat_entries = Vec::new();
    for (name, entries) in &doc.categories {
        let items: Vec<Value> = entries
            .iter()
            .map(|e| {
                if e.weight == 1 {
                    Value::String(e.text.clone())
                } else {
                    Value::Object(vec![
                        ("text".to_string(), Value::String(e.text.clone())),
                        ("weight".to_string(), Value::Number(e.weight as f64)),
                    ])
                }
            })
            .collect();
        cat_entries.push((name.clone(), Value::Array(items)));
    }

    let mut top = Vec::new();
    if let Some(start) = &doc.start {
        top.push(("start".to_string(), Value::String(start.clone())));
    }
    top.push(("categories".to_string(), Value::Object(cat_entries)));

    json::write(&Value::Object(top))
}
