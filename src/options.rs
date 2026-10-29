use crate::mapping::ControlPlan;
use indexmap::IndexMap;
use std::fmt;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsDocument {
    pub lines: Vec<String>,
    pub bindings: IndexMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsError(pub String);
impl fmt::Display for OptionsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for OptionsError {}

pub fn parse_options(bytes: &[u8]) -> Result<OptionsDocument, OptionsError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| OptionsError(format!("options.txt is not UTF-8: {error}")))?;
    let mut document = OptionsDocument::default();
    let bytes = text.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let mut end = start;
        while end < bytes.len() && bytes[end] != b'\r' && bytes[end] != b'\n' {
            end += 1;
        }
        if end < bytes.len() {
            end += 1;
            if bytes[end - 1] == b'\r' && bytes.get(end) == Some(&b'\n') {
                end += 1;
            }
        }
        let line = &text[start..end];
        if let Some((name, value)) = line.split_once(':') {
            if name.starts_with("key_") {
                if document.bindings.contains_key(name) {
                    return Err(OptionsError(format!("duplicate control entry: {name}")));
                }
                document
                    .bindings
                    .insert(name.into(), value.trim_end_matches(['\r', '\n']).into());
            }
        }
        document.lines.push(line.into());
        start = end;
    }
    Ok(document)
}

pub fn render_options(options: &OptionsDocument, plan: &ControlPlan) -> Vec<u8> {
    assert!(
        plan.conflicts.is_empty(),
        "cannot render conflicted controls"
    );
    let mut rendered = String::new();
    for line in &options.lines {
        if let Some((name, _)) = line.split_once(':') {
            if let Some(value) = plan.updates.get(name) {
                let ending = if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else if line.ends_with('\r') {
                    "\r"
                } else {
                    ""
                };
                rendered.push_str(name);
                rendered.push(':');
                rendered.push_str(value);
                rendered.push_str(ending);
                continue;
            }
        }
        rendered.push_str(line);
    }
    let ending = if options.lines.iter().any(|line| line.ends_with("\r\n")) {
        "\r\n"
    } else if options.lines.iter().any(|line| line.ends_with('\n')) {
        "\n"
    } else if options.lines.iter().any(|line| line.ends_with('\r')) {
        "\r"
    } else {
        "\n"
    };
    let mut added = false;
    for (name, value) in &plan.updates {
        if options.bindings.contains_key(name) {
            continue;
        }
        if !added && !rendered.is_empty() && !rendered.ends_with(['\r', '\n']) {
            rendered.push_str(ending);
        }
        rendered.push_str(name);
        rendered.push(':');
        rendered.push_str(value);
        rendered.push_str(ending);
        added = true;
    }
    rendered.into_bytes()
}

/// Prepare windowed mode independently of keyboard controls.
pub fn render_windowed_options(bytes: &[u8]) -> Result<Vec<u8>, OptionsError> {
    let document = parse_options(bytes)?;
    let mut seen = false;
    let mut output = String::new();
    for line in &document.lines {
        let body = line.trim_end_matches(['\r', '\n']);
        if body == "fullscreen" || body.starts_with("fullscreen:") {
            if seen {
                return Err(OptionsError("duplicate fullscreen entry".into()));
            }
            seen = true;
            match body.strip_prefix("fullscreen:") {
                Some("true") => {
                    output.push_str("fullscreen:false");
                    output.push_str(&line[body.len()..]);
                }
                Some("false") => output.push_str(line),
                _ => return Err(OptionsError("invalid fullscreen boolean".into())),
            }
        } else {
            output.push_str(line);
        }
    }
    if !seen {
        let ending = document
            .lines
            .iter()
            .find_map(|line| {
                if line.ends_with("\r\n") {
                    Some("\r\n")
                } else if line.ends_with('\n') {
                    Some("\n")
                } else if line.ends_with('\r') {
                    Some("\r")
                } else {
                    None
                }
            })
            .unwrap_or("\n");
        let final_newline = output.ends_with(['\r', '\n']);
        if !output.is_empty() && !final_newline {
            output.push_str(ending);
        }
        output.push_str("fullscreen:false");
        if final_newline {
            output.push_str(ending);
        }
    }
    Ok(output.into_bytes())
}

#[cfg(test)]
mod windowed_tests {
    use super::*;
    #[test]
    fn windowed_preserves_lines_and_never_enters_bindings() {
        for (before, after) in [
            ("fullscreen:true\nother:1\n", "fullscreen:false\nother:1\n"),
            ("fullscreen:true\r\n", "fullscreen:false\r\n"),
            ("fullscreen:true\r", "fullscreen:false\r"),
            ("fullscreen:true", "fullscreen:false"),
            ("fullscreen:false", "fullscreen:false"),
            ("other:1\r\n", "other:1\r\nfullscreen:false\r\n"),
            ("other:1", "other:1\nfullscreen:false"),
            ("", "fullscreen:false"),
        ] {
            let result = render_windowed_options(before.as_bytes()).unwrap();
            assert_eq!(result, after.as_bytes(), "{before:?}");
            let doc = parse_options(&result).unwrap();
            assert!(!doc.bindings.contains_key("fullscreen"));
            let plan = crate::mapping::plan_controls(&doc, &Default::default(), None).unwrap();
            assert_eq!(render_options(&doc, &plan), result);
        }
    }
    #[test]
    fn windowed_rejects_ambiguous_or_malformed_values() {
        for text in [
            "fullscreen:true\nfullscreen:false\n",
            "fullscreen:yes",
            "fullscreen:",
            "fullscreen",
            "fullscreen: true",
            "fullscreen:true:false",
        ] {
            assert!(render_windowed_options(text.as_bytes()).is_err(), "{text}");
        }
    }
}
