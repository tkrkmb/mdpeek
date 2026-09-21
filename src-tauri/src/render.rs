use comrak::{markdown_to_html, Options};

/// 本文をGFM相当のHTMLに変換する。生HTMLは描画しない。
pub fn to_html(markdown: &str) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.math_dollars = true;
    options.extension.header_id_prefix = Some(String::new());
    options.render.sourcepos = true;
    // options.render.unsafe は既定の false のままにする（生HTMLは描画しない）
    markdown_to_html(markdown, &options)
}

#[cfg(test)]
mod tests {
    use super::to_html;

    #[test]
    fn keeps_source_positions() {
        let html = to_html("# title\n\ntext\n");
        assert!(html.contains("data-sourcepos=\"1:1-1:7\""), "{html}");
    }

    #[test]
    fn escapes_raw_html() {
        let html = to_html("<script>alert(1)</script>\n");
        assert!(!html.contains("<script>"), "{html}");
    }

    #[test]
    fn renders_gfm_extensions() {
        let html = to_html("- [ ] todo\n");
        assert!(html.contains("type=\"checkbox\""), "{html}");
    }
}
