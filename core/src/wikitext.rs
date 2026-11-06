//! Small, forgiving helpers for reading MediaWiki wikitext.
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::BTreeMap;

macro_rules! re {
    ($p:expr) => {{
        static R: Lazy<Regex> = Lazy::new(|| Regex::new($p).unwrap());
        &*R
    }};
}

/// Turn wikitext into plain text. Icon alt text is kept as `[Alt]`.
pub fn clean(s: &str) -> String {
    let s = re!(r"</?font[^>]*>").replace_all(s, "");
    let s = re!(r"<br\s*/?>").replace_all(&s, " / ");
    let s = re!(r"'{2,3}").replace_all(&s, "");
    let s = re!(r"\[\[File:[^\]|]*((?:\|[^\]|]*)+)\]\]").replace_all(&s, |c: &Captures| {
        let last = c[1].rsplit('|').next().unwrap_or("").trim().to_string();
        let is_opt = last.contains('=')
            || re!(r"^(frameless|thumb|center|left|right|\d*x?\d+px)$").is_match(&last);
        if is_opt || last.is_empty() {
            String::new()
        } else {
            format!("[{}]", last)
        }
    });
    let s = re!(r"\[\[File:[^\]]*\]\]").replace_all(&s, "");
    let s = re!(r"\[\[(?:[^\]|]*\|)?([^\]]*)\]\]").replace_all(&s, "$1");
    let s = re!(r"<[^>]+>").replace_all(&s, "");
    s.trim().to_string()
}

/// Link targets (`[[Target|label]]` -> `Target`), skipping File/Category links.
pub fn links(s: &str) -> Vec<String> {
    re!(r"\[\[([^\]|#]+)(?:#[^\]|]*)?(?:\|[^\]]*)?\]\]")
        .captures_iter(s)
        .map(|c| c[1].trim().to_string())
        .filter(|t| {
            let l = t.to_lowercase();
            !t.is_empty() && !l.starts_with("file:") && !l.starts_with("category:")
        })
        .collect()
}

/// First number in a cell (`<font>+50</font>` -> 50, `1,200` -> 1200).
pub fn num(s: &str) -> Option<f64> {
    let c = clean(s);
    re!(r"[-+]?\d[\d,]*\.?\d*")
        .find(&c)
        .and_then(|m| m.as_str().replace(',', "").parse::<f64>().ok())
}

/// `{{Infobox name ... }}` arguments (single-line `|key = value`).
pub fn infobox(text: &str, name: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let pat = format!(r"(?s)\{{\{{\s*{}\s*\n(.*?)\n\}}\}}", regex::escape(name));
    let Ok(re) = Regex::new(&pat) else { return out };
    let Some(m) = re.captures(text) else { return out };
    let line = re!(r"^\|\s*([^=|]+?)\s*=\s*(.*)$");
    for l in m[1].lines() {
        if let Some(c) = line.captures(l) {
            out.insert(c[1].trim().to_string(), c[2].trim().to_string());
        }
    }
    out
}

/// Top-level `==Heading==` sections in page order (sub-headings stay inside).
pub fn sections(text: &str) -> Vec<(String, String)> {
    let re = re!(r"(?m)^==\s*([^=\n]+?)\s*==[ \t]*$");
    let heads: Vec<_> = re.captures_iter(text).collect();
    let mut out = Vec::new();
    for (i, c) in heads.iter().enumerate() {
        let start = c.get(0).unwrap().end();
        let end = heads.get(i + 1).map(|n| n.get(0).unwrap().start()).unwrap_or(text.len());
        out.push((c[1].trim().to_string(), text[start..end].to_string()));
    }
    out
}

pub fn section<'a>(secs: &'a [(String, String)], name: &str) -> &'a str {
    secs.iter().find(|(n, _)| n == name).map(|(_, b)| b.as_str()).unwrap_or("")
}

/// Rows of a wikitable as raw cell strings. Cell attributes are stripped.
pub fn table_rows(text: &str) -> Vec<Vec<String>> {
    let sep = re!(r"(?m)^\|-[ \t]*$");
    let attr = re!(r#"^\s*(?:[\w-]+="[^"]*"\s*)+\|"#);
    let mut rows = Vec::new();
    for block in sep.split(text) {
        let mut cells: Vec<Vec<String>> = Vec::new();
        let mut open = false;
        for line in block.lines() {
            if line.starts_with("|}") {
                break;
            }
            let first = line.chars().next();
            if matches!(first, Some('|') | Some('!')) && !line.starts_with("{|") && !line.starts_with("|+") {
                let mut body = &line[1..];
                if let Some(m) = attr.find(body) {
                    if body[m.end()..].chars().next() != Some('|') {
                        body = &body[m.end()..];
                    }
                }
                cells.push(vec![body.to_string()]);
                open = true;
            } else if open {
                cells.last_mut().unwrap().push(line.to_string());
            }
        }
        if !cells.is_empty() {
            rows.push(cells.into_iter().map(|c| c.join("\n").trim().to_string()).collect());
        }
    }
    rows
}

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in s.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            out.push(ch);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// File name of the first `[[File:Name.png|...]]` in a cell.
pub fn file_name(s: &str) -> Option<String> {
    re!(r"\[\[File:([^|\]]+)").captures(s).map(|c| c[1].trim().to_string()).filter(|n| !n.is_empty())
}

/// An infobox `image`/`icon` value: either `Name.png` or `[[File:Name.png|..]]`.
pub fn plain_file(v: &str) -> Option<String> {
    let v = v.trim();
    if v.is_empty() {
        return None;
    }
    file_name(v).or_else(|| {
        let n = v.trim_start_matches("File:").trim();
        (!n.is_empty() && !n.contains("{{")).then(|| n.to_string())
    })
}

/// Icon file of an item page. Tries, in order: the infobox `icon` parameter (also the odd
/// `|Name icon.png (Icon is hidden ...)` line weapon pages use), any `|icon =` on the page,
/// a `[[File:..|link=<this page>]]` image (crafting tables), then the infobox `image`.
pub fn infobox_icon(text: &str, titles: &[&str]) -> Option<String> {
    let kv = re!(r"^\|\s*([^=|]+?)\s*=\s*(.*)$");
    let loose = re!(r"(?i)^\|\s*([^=|]+?\.(?:png|jpe?g|gif|webp))\b");
    let clean_val = |v: &str| plain_file(v.trim().trim_end_matches('}').trim());

    let (mut icon, mut image) = (None, None);
    if let Some(m) = re!(r"(?i)\{\{\s*infobox[ _][A-Za-z ]+").find(text) {
        let tail = &text[m.end()..];
        let end = tail.find("\n}}").unwrap_or_else(|| tail.len().min(6000));
        for l in tail[..end].lines() {
            if let Some(c) = kv.captures(l) {
                match c[1].trim().to_lowercase().as_str() {
                    "icon" if icon.is_none() => icon = clean_val(&c[2]),
                    "image" if image.is_none() => image = clean_val(&c[2]),
                    _ => {}
                }
            } else if icon.is_none() {
                if let Some(c) = loose.captures(l) {
                    icon = Some(c[1].trim().to_string());
                }
            }
        }
    }
    if icon.is_some() {
        return icon;
    }
    if let Some(c) = re!(r"(?i)\|\s*icon\s*=\s*([^\n|}]+)").captures(text) {
        if let Some(f) = clean_val(&c[1]) {
            return Some(f);
        }
    }
    let wanted: Vec<String> = titles.iter().map(|t| t.trim().to_lowercase()).collect();
    let mut linked: Vec<String> = Vec::new();
    for c in re!(r"\[\[File:([^|\]]+)((?:\|[^\]]*)?)\]\]").captures_iter(text) {
        let opts = c[2].to_lowercase();
        if let Some(pos) = opts.find("link=") {
            let target = opts[pos + 5..].split('|').next().unwrap_or("").trim().to_string();
            if wanted.contains(&target) {
                linked.push(c[1].trim().to_string());
            }
        }
    }
    linked
        .iter()
        .find(|n| n.to_lowercase().contains("icon"))
        .or_else(|| linked.first())
        .cloned()
        .or(image)
}
