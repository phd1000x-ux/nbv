/// Decode the small set of HTML entities pandas `.to_html()` emits, plus
/// numeric character references (`&#NN;`). Unknown entities and stray `&`
/// without a closing `;` are left untouched.
fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let Some(semi) = rest.find(';') else {
            out.push_str(rest);
            rest = "";
            break;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "#39" | "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix('#')
                .and_then(|n| n.parse::<u32>().ok())
                .and_then(char::from_u32),
        };
        match decoded {
            Some(c) => {
                out.push(c);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_named_entities() {
        assert_eq!(decode_entities("&lt;a&gt;"), "<a>");
        assert_eq!(decode_entities("x &amp; y"), "x & y");
        assert_eq!(decode_entities("say &quot;hi&quot;"), "say \"hi\"");
        assert_eq!(decode_entities("it&#39;s"), "it's");
        assert_eq!(decode_entities("a&nbsp;b"), "a b");
    }

    #[test]
    fn decodes_numeric_entities() {
        assert_eq!(decode_entities("&#65;&#66;"), "AB");
    }

    #[test]
    fn leaves_unknown_or_unterminated_alone() {
        assert_eq!(decode_entities("100% &done"), "100% &done");
        assert_eq!(decode_entities("&notanentity;"), "&notanentity;");
        assert_eq!(decode_entities("plain text"), "plain text");
    }
}
