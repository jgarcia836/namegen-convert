// Recursive name sampling: expands the `start` category (or any named
// category) into generated text by substituting `{placeholder}` references
// with a random pick from the referenced category, weighted by each entry's
// weight. `\{` and `\}` in an entry are unescaped into a literal brace
// instead of being read as a placeholder.
//
// Grammars can be cyclic (a typo, or a deliberate mutual reference), so
// expansion is bounded by depth instead of trusting the input to terminate
// on its own.

use crate::model::Document;

const MAX_DEPTH: u32 = 64;

pub struct Rng(u64);

impl Rng {
    // std has no RNG in the standard library, but RandomState's hasher is
    // seeded from the OS once per process, so hashing an ambient value
    // through it gives a reasonably unpredictable, non-zero seed without
    // pulling in a crate.
    pub fn from_entropy() -> Rng {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};
        let mut hasher = RandomState::new().build_hasher();
        std::time::Instant::now().hash(&mut hasher);
        Rng::from_seed(hasher.finish())
    }

    pub fn from_seed(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }

    // splitmix64
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    // Uniform in [0, bound). bound is always >= 1 for our callers.
    fn below(&mut self, bound: u32) -> u32 {
        (self.next_u64() % bound as u64) as u32
    }
}

// Samples the document's start category. Returns an error if there is no
// start category, a referenced category is missing, a category has no
// entries, or expansion recurses past MAX_DEPTH (almost certainly a cycle).
pub fn sample(doc: &Document, rng: &mut Rng) -> Result<String, String> {
    let start = doc.start.as_ref().ok_or("document has no start category")?;
    expand(doc, start, rng, 0)
}

fn expand(doc: &Document, category: &str, rng: &mut Rng, depth: u32) -> Result<String, String> {
    if depth > MAX_DEPTH {
        return Err(format!(
            "recursion limit exceeded expanding '{{{}}}'; check for a reference cycle",
            category
        ));
    }

    let entries = doc
        .categories
        .iter()
        .find(|(name, _)| name == category)
        .map(|(_, entries)| entries)
        .ok_or_else(|| format!("category '{}' is not defined", category))?;

    if entries.is_empty() {
        return Err(format!("category '{}' has no entries to sample", category));
    }

    let total_weight: u32 = entries.iter().map(|e| e.weight).sum();
    let mut pick = rng.below(total_weight);
    let entry = entries
        .iter()
        .find(|e| {
            if pick < e.weight {
                true
            } else {
                pick -= e.weight;
                false
            }
        })
        .expect("total_weight covers every entry's weight range");

    render(doc, &entry.text, rng, depth)
}

// Substitutes unescaped `{name}` placeholders with a recursive sample of
// `name`, and unescapes `\{` / `\}` into a literal brace. Kept in sync with
// model::placeholders, which finds the same placeholders without expanding
// them.
fn render(doc: &Document, text: &str, rng: &mut Rng, depth: u32) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = text;
    loop {
        let Some(idx) = rest.find(|c| c == '{' || c == '\\') else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..idx]);
        let tail = &rest[idx..];
        if tail.starts_with("\\{") || tail.starts_with("\\}") {
            result.push_str(&tail[1..2]);
            rest = &tail[2..];
            continue;
        }
        if tail.starts_with('\\') {
            result.push('\\');
            rest = &tail[1..];
            continue;
        }
        let Some(rel_end) = tail[1..].find('}') else {
            result.push_str(tail);
            break;
        };
        let end = 1 + rel_end;
        let name = &tail[1..end];
        let is_ref = !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_');
        if is_ref && doc.has_category(name) {
            result.push_str(&expand(doc, name, rng, depth + 1)?);
        } else {
            result.push_str(&tail[..=end]);
        }
        rest = &tail[end + 1..];
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngt;

    fn doc(src: &str) -> Document {
        let (mut doc, mut issues) = ngt::parse(src).expect("valid ngt fixture");
        crate::model::finalize(&mut doc, &mut issues);
        doc
    }

    #[test]
    fn expands_placeholders_recursively() {
        let doc = doc("first = Anna\nlast = Smith\nname = {first} {last}\n");
        let mut rng = Rng::from_seed(1);
        assert_eq!(sample(&doc, &mut rng).unwrap(), "Anna Smith");
    }

    #[test]
    fn same_seed_is_reproducible() {
        let doc = doc("first = Anna, Beth, Clara, Dana\nname = {first}\n");
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);
        let mut results_a = Vec::new();
        let mut results_b = Vec::new();
        for _ in 0..20 {
            results_a.push(sample(&doc, &mut a).unwrap());
            results_b.push(sample(&doc, &mut b).unwrap());
        }
        assert_eq!(results_a, results_b);
    }

    #[test]
    fn only_defined_entries_are_selected() {
        let doc = doc("first = Rare:1, Common:1000\nname = {first}\n");
        let mut rng = Rng::from_seed(7);
        for _ in 0..50 {
            let result = sample(&doc, &mut rng).unwrap();
            assert!(result == "Rare" || result == "Common");
        }
    }

    #[test]
    fn cyclic_reference_is_rejected() {
        let doc = doc("a = {b}\nb = {a}\nname = {a}\n");
        let mut rng = Rng::from_seed(1);
        let err = sample(&doc, &mut rng).unwrap_err();
        assert!(err.contains("recursion limit"), "unexpected error: {}", err);
    }

    #[test]
    fn unresolved_placeholder_is_left_literal() {
        let doc = doc("name = hello {ghost}\n");
        let mut rng = Rng::from_seed(1);
        assert_eq!(sample(&doc, &mut rng).unwrap(), "hello {ghost}");
    }

    #[test]
    fn escaped_braces_are_kept_literal() {
        let doc = doc("name = \\{5\\}\n");
        let mut rng = Rng::from_seed(1);
        assert_eq!(sample(&doc, &mut rng).unwrap(), "{5}");
    }

    #[test]
    fn escaped_brace_next_to_a_real_placeholder_only_escapes_itself() {
        let doc = doc("first = Anna\nname = \\{first\\} {first}\n");
        let mut rng = Rng::from_seed(1);
        assert_eq!(sample(&doc, &mut rng).unwrap(), "{first} Anna");
    }

    #[test]
    fn missing_start_category_is_an_error() {
        let mut doc = Document::default();
        doc.start = None;
        let mut rng = Rng::from_seed(1);
        assert!(sample(&doc, &mut rng).is_err());
    }
}
