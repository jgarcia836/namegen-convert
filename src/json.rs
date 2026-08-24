// Minimal JSON reader/writer. The project has a zero-dependency rule, and the
// on-disk .ngj format is deliberately small (objects, arrays, strings,
// numbers), so a hand-rolled parser is less trouble than it sounds.
//
// Object keys are kept in a Vec instead of a HashMap so that insertion order
// is preserved on write and duplicate keys are visible to the caller instead
// of silently overwriting each other.

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Vec<(String, Value)>> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    let value = parse_value(&chars, &mut pos)?;
    skip_ws(&chars, &mut pos);
    if pos != chars.len() {
        return Err(format!("unexpected trailing content at position {}", pos));
    }
    Ok(value)
}

fn skip_ws(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
}

fn parse_value(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    skip_ws(chars, pos);
    match chars.get(*pos) {
        Some('{') => parse_object(chars, pos),
        Some('[') => parse_array(chars, pos),
        Some('"') => Ok(Value::String(parse_string(chars, pos)?)),
        Some('t') => parse_literal(chars, pos, "true", Value::Bool(true)),
        Some('f') => parse_literal(chars, pos, "false", Value::Bool(false)),
        Some('n') => parse_literal(chars, pos, "null", Value::Null),
        Some(c) if *c == '-' || c.is_ascii_digit() => parse_number(chars, pos),
        Some(c) => Err(format!("unexpected character '{}' at position {}", c, pos)),
        None => Err("unexpected end of input".to_string()),
    }
}

fn parse_literal(chars: &[char], pos: &mut usize, lit: &str, value: Value) -> Result<Value, String> {
    let lit_chars: Vec<char> = lit.chars().collect();
    if *pos + lit_chars.len() <= chars.len() && chars[*pos..*pos + lit_chars.len()] == lit_chars[..] {
        *pos += lit_chars.len();
        Ok(value)
    } else {
        Err(format!("invalid literal at position {}", pos))
    }
}

fn parse_object(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    *pos += 1; // consume '{'
    let mut entries = Vec::new();
    skip_ws(chars, pos);
    if chars.get(*pos) == Some(&'}') {
        *pos += 1;
        return Ok(Value::Object(entries));
    }
    loop {
        skip_ws(chars, pos);
        if chars.get(*pos) != Some(&'"') {
            return Err(format!("expected string key at position {}", pos));
        }
        let key = parse_string(chars, pos)?;
        skip_ws(chars, pos);
        if chars.get(*pos) != Some(&':') {
            return Err(format!("expected ':' at position {}", pos));
        }
        *pos += 1;
        let value = parse_value(chars, pos)?;
        entries.push((key, value));
        skip_ws(chars, pos);
        match chars.get(*pos) {
            Some(',') => {
                *pos += 1;
            }
            Some('}') => {
                *pos += 1;
                break;
            }
            _ => return Err(format!("expected ',' or '}}' at position {}", pos)),
        }
    }
    Ok(Value::Object(entries))
}

fn parse_array(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    *pos += 1; // consume '['
    let mut items = Vec::new();
    skip_ws(chars, pos);
    if chars.get(*pos) == Some(&']') {
        *pos += 1;
        return Ok(Value::Array(items));
    }
    loop {
        let value = parse_value(chars, pos)?;
        items.push(value);
        skip_ws(chars, pos);
        match chars.get(*pos) {
            Some(',') => {
                *pos += 1;
            }
            Some(']') => {
                *pos += 1;
                break;
            }
            _ => return Err(format!("expected ',' or ']' at position {}", pos)),
        }
    }
    Ok(Value::Array(items))
}

fn parse_string(chars: &[char], pos: &mut usize) -> Result<String, String> {
    *pos += 1; // consume opening quote
    let mut result = String::new();
    loop {
        match chars.get(*pos) {
            Some('"') => {
                *pos += 1;
                break;
            }
            Some('\\') => {
                *pos += 1;
                match chars.get(*pos) {
                    Some('"') => result.push('"'),
                    Some('\\') => result.push('\\'),
                    Some('/') => result.push('/'),
                    Some('b') => result.push('\u{0008}'),
                    Some('f') => result.push('\u{000C}'),
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('u') => {
                        let code = parse_hex4(chars, *pos + 1)?;
                        result.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                        *pos += 4;
                    }
                    _ => return Err(format!("invalid escape sequence at position {}", pos)),
                }
                *pos += 1;
            }
            Some(c) => {
                result.push(*c);
                *pos += 1;
            }
            None => return Err("unterminated string".to_string()),
        }
    }
    Ok(result)
}

fn parse_hex4(chars: &[char], start: usize) -> Result<u32, String> {
    if start + 4 > chars.len() {
        return Err("truncated unicode escape".to_string());
    }
    let hex: String = chars[start..start + 4].iter().collect();
    u32::from_str_radix(&hex, 16).map_err(|_| format!("invalid unicode escape '{}'", hex))
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<Value, String> {
    let start = *pos;
    if chars.get(*pos) == Some(&'-') {
        *pos += 1;
    }
    while chars.get(*pos).map_or(false, |c| c.is_ascii_digit()) {
        *pos += 1;
    }
    if chars.get(*pos) == Some(&'.') {
        *pos += 1;
        while chars.get(*pos).map_or(false, |c| c.is_ascii_digit()) {
            *pos += 1;
        }
    }
    if matches!(chars.get(*pos), Some('e') | Some('E')) {
        *pos += 1;
        if matches!(chars.get(*pos), Some('+') | Some('-')) {
            *pos += 1;
        }
        while chars.get(*pos).map_or(false, |c| c.is_ascii_digit()) {
            *pos += 1;
        }
    }
    let text: String = chars[start..*pos].iter().collect();
    text.parse::<f64>().map(Value::Number).map_err(|_| format!("invalid number '{}'", text))
}

pub fn write(value: &Value) -> String {
    let mut out = String::new();
    write_value(value, 0, &mut out);
    out
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn write_value(value: &Value, level: usize, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                out.push_str(&(*n as i64).to_string());
            } else {
                out.push_str(&n.to_string());
            }
        }
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                indent(level + 1, out);
                write_value(item, level + 1, out);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(level, out);
            out.push(']');
        }
        Value::Object(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (i, (key, val)) in entries.iter().enumerate() {
                indent(level + 1, out);
                write_string(key, out);
                out.push_str(": ");
                write_value(val, level + 1, out);
                if i + 1 < entries.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            indent(level, out);
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
