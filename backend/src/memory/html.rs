//! Minimal HTML → plain-text stripper for embedding/search indexing.
//!
//! No external dependency: byte-scan based, handles common entities.
//! Used so rich-text (Tiptap HTML) memories don't leak tags into embeddings
//! or full-text search while memory.content keeps the full HTML.

/// Strip HTML tags from a string and unescape common entities.
pub fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let bytes = html.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Skip until matching '>'.
            while i < bytes.len() && bytes[i] != b'>' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // consume '>'
            }
            continue;
        }
        if bytes[i] == b'&' {
            // Try to unescape a known entity.
            if let Some((ch, consumed)) = unescape_entity(&html[i..]) {
                out.push(ch);
                i += consumed;
                continue;
            }
        }
        // Safe UTF-8 boundary push.
        let ch_len = utf8_len(bytes[i]);
        let end = (i + ch_len).min(bytes.len());
        out.push_str(&html[i..end]);
        i = end;
    }
    out
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else if first >> 3 == 0b11110 {
        4
    } else {
        1 // invalid continuation byte: advance 1
    }
}

fn unescape_entity(s: &str) -> Option<(char, usize)> {
    let rest = s.strip_prefix('&')?;
    if rest.strip_prefix("amp;").is_some() {
        return Some(('&', 5));
    }
    if rest.strip_prefix("lt;").is_some() {
        return Some(('<', 4));
    }
    if rest.strip_prefix("gt;").is_some() {
        return Some(('>', 4));
    }
    if rest.strip_prefix("quot;").is_some() {
        return Some(('"', 6));
    }
    if rest.strip_prefix("apos;").is_some() {
        return Some(('\'', 6));
    }
    if rest.strip_prefix("nbsp;").is_some() {
        return Some((' ', 6));
    }
    None
}
