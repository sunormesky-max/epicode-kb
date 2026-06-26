use epicode_kb::memory::html::strip_tags;

#[test]
fn test_strip_basic_tags() {
    assert_eq!(strip_tags("<h1>Title</h1><p>Body text</p>"), "TitleBody text");
}

#[test]
fn test_strip_preserves_text_between_tags() {
    assert_eq!(strip_tags("<ul><li>One</li><li>Two</li></ul>"), "OneTwo");
}

#[test]
fn test_strip_unescapes_common_entities() {
    assert_eq!(
        strip_tags("a &amp; b &lt;tag&gt; &quot;q&quot;"),
        "a & b <tag> \"q\""
    );
}

#[test]
fn test_strip_plain_text_unchanged() {
    assert_eq!(strip_tags("no html here"), "no html here");
}

#[test]
fn test_strip_empty() {
    assert_eq!(strip_tags(""), "");
    let ws = strip_tags("   <p>  </p>  ");
    assert_eq!(ws.trim(), "");
}

#[test]
fn test_strip_handles_unclosed_tag() {
    assert_eq!(strip_tags("text <b bold"), "text ");
}

#[test]
fn test_strip_preserves_utf8() {
    assert_eq!(strip_tags("<p>中文测试 emoji 🎉</p>"), "中文测试 emoji 🎉");
}
