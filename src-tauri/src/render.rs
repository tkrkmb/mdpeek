use comrak::nodes::NodeValue;
use comrak::{format_html, parse_document, Arena, Options};
use yaml_rust2::{Yaml, YamlLoader};

const FRONT_MATTER_DELIMITER: &str = "---";

/// 本文をGFM相当のHTMLに変換する。生HTMLは描画しない。
pub fn to_html(markdown: &str) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.tasklist = true;
    options.extension.autolink = true;
    options.extension.math_dollars = true;
    options.extension.header_id_prefix = Some(String::new());
    options.extension.footnotes = true;
    options.extension.alerts = true;
    options.extension.front_matter_delimiter = Some(FRONT_MATTER_DELIMITER.to_string());
    options.render.sourcepos = true;
    // options.render.unsafe は既定の false のままにする（生HTMLは描画しない）

    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options);
    let mut html = String::new();
    // comrakはフロントマターを出力しないので、表にして本文の前に置く
    if let Some(node) = root.first_child() {
        let data = node.data.borrow();
        if let NodeValue::FrontMatter(text) = &data.value {
            html.push_str(&front_matter(text, &data.sourcepos.to_string()));
        }
    }
    format_html(root, &options, &mut html).expect("writing to a String does not fail");
    html
}

/// 区切りの行を除いた、フロントマターのYAMLを取り出す
fn front_matter_yaml(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().skip(1).collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.last().is_some_and(|line| line.trim_end() == FRONT_MATTER_DELIMITER) {
        lines.pop();
    }
    lines.iter().map(|line| format!("{line}\n")).collect()
}

/// GitHubと同じく、最上位のキーを見出しの行に、値をその下の行に並べた表にする。
/// YAMLとして読めないとき、または最上位がマッピングでないときは、元のテキストをコードブロックにする。
fn front_matter(text: &str, sourcepos: &str) -> String {
    let yaml = front_matter_yaml(text);
    match YamlLoader::load_from_str(&yaml) {
        Ok(documents) => match documents.first() {
            Some(Yaml::Hash(hash)) => {
                format!("<table data-sourcepos=\"{sourcepos}\">{}</table>\n", hash_rows(hash))
            }
            _ => code_block(&yaml, sourcepos),
        },
        Err(_) => code_block(&yaml, sourcepos),
    }
}

fn code_block(yaml: &str, sourcepos: &str) -> String {
    format!(
        "<pre data-sourcepos=\"{sourcepos}\"><code class=\"language-yaml\">{}</code></pre>\n",
        escape(yaml)
    )
}

fn hash_rows(hash: &yaml_rust2::yaml::Hash) -> String {
    let head: String = hash.keys().map(|key| format!("<th>{}</th>", escape(&scalar(key)))).collect();
    let cells: String = hash.values().map(|value| format!("<td>{}</td>", value_html(value))).collect();
    format!("<thead><tr>{head}</tr></thead><tbody><tr>{cells}</tr></tbody>")
}

fn value_html(value: &Yaml) -> String {
    match value {
        Yaml::Hash(hash) => format!("<table>{}</table>", hash_rows(hash)),
        Yaml::Array(items) => {
            let cells: String = items.iter().map(|item| format!("<td>{}</td>", value_html(item))).collect();
            format!("<table><tbody><tr>{cells}</tr></tbody></table>")
        }
        other => format!("<div>{}</div>", escape(&scalar(other))),
    }
}

fn scalar(value: &Yaml) -> String {
    match value {
        Yaml::String(text) | Yaml::Real(text) => text.clone(),
        Yaml::Integer(number) => number.to_string(),
        Yaml::Boolean(flag) => flag.to_string(),
        _ => String::new(),
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
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

    #[test]
    fn renders_front_matter_as_a_table_before_the_body() {
        let html = to_html("---\ntitle: Notes\ntags: [a, b]\n---\n\n# Heading\n");
        assert!(
            html.starts_with("<table data-sourcepos=\"1:1-4:3\"><thead><tr><th>title</th><th>tags</th></tr></thead>"),
            "{html}"
        );
        assert!(html.contains("<td><div>Notes</div></td>"), "{html}");
        assert!(
            html.contains("<td><table><tbody><tr><td><div>a</div></td><td><div>b</div></td></tr></tbody></table></td>"),
            "{html}"
        );
        // 本文の行番号は、フロントマターの分だけ後ろにずれたまま
        assert!(html.contains("<h1 id=\"heading\" data-sourcepos=\"6:1-6:9\""), "{html}");
        assert!(!html.contains("<hr"), "{html}");
    }

    #[test]
    fn nests_mappings_in_front_matter() {
        let html = to_html("---\nauthor:\n  name: A\n---\n");
        assert!(
            html.contains("<td><table><thead><tr><th>name</th></tr></thead><tbody><tr><td><div>A</div></td></tr></tbody></table></td>"),
            "{html}"
        );
    }

    #[test]
    fn escapes_front_matter_values() {
        let html = to_html("---\n\"<b>\": \"<script>x</script>\"\n---\n");
        assert!(html.contains("<th>&lt;b&gt;</th>"), "{html}");
        assert!(!html.contains("<script>"), "{html}");
    }

    #[test]
    fn shows_unreadable_front_matter_as_code() {
        let html = to_html("---\ntitle: [unclosed\n---\n");
        assert!(
            html.starts_with("<pre data-sourcepos=\"1:1-3:3\"><code class=\"language-yaml\">title: [unclosed\n</code></pre>"),
            "{html}"
        );
    }

    #[test]
    fn shows_non_mapping_front_matter_as_code() {
        let html = to_html("---\n- a\n- b\n---\n");
        assert!(html.contains("<code class=\"language-yaml\">- a\n- b\n</code>"), "{html}");
    }

    #[test]
    fn detects_front_matter_without_a_trailing_newline() {
        // Neovimからは、行を改行でつないだだけの本文が届く
        let html = to_html("---\ntitle: Notes\n---");
        assert!(html.contains("<th>title</th>"), "{html}");
    }

    #[test]
    fn renders_footnotes_and_alerts() {
        let html = to_html("text[^1]\n\n[^1]: note\n\n> [!NOTE]\n> body\n");
        assert!(html.contains("href=\"#fn-1\""), "{html}");
        assert!(html.contains("class=\"footnotes\""), "{html}");
        assert!(html.contains("markdown-alert markdown-alert-note"), "{html}");
    }
}
