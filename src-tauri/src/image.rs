use std::path::{Path, PathBuf};

use percent_encoding::percent_decode_str;

/// 表示してよい画像の拡張子
const EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "svg"];

/// 文書のディレクトリを基準にパスを解決し、実体のパスを返す。
/// 存在して、拡張子が許可されたものである場合だけ成功する。
pub fn resolve(base: &Path, raw: &str) -> Result<PathBuf, String> {
    let decoded = percent_decode_str(raw).decode_utf8_lossy().to_string();
    let candidate = base.join(decoded);

    // シンボリックリンクを実体のパスに解決する（存在しなければ失敗する）
    let resolved = std::fs::canonicalize(&candidate)
        .map_err(|err| format!("cannot resolve {}: {err}", candidate.display()))?;
    if !resolved.is_file() {
        return Err(format!("{} is not a file", resolved.display()));
    }

    let extension = resolved
        .extension()
        .map(|found| found.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if !EXTENSIONS.contains(&extension.as_str()) {
        return Err(format!("{extension} is not an allowed image type"));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::resolve;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn workspace(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!("mdpeek-image-{name}"));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("img")).expect("a test directory");
        directory
    }

    fn write(path: &Path) {
        fs::write(path, b"x").expect("a test file");
    }

    #[test]
    fn resolves_a_relative_image() {
        let base = workspace("relative");
        write(&base.join("img/a.png"));
        let resolved = resolve(&base, "img/a.png").expect("a path");
        assert_eq!(resolved, fs::canonicalize(base.join("img/a.png")).unwrap());
    }

    #[test]
    fn decodes_percent_encoding() {
        let base = workspace("encoded");
        write(&base.join("img/a b.png"));
        assert!(resolve(&base, "img/a%20b.png").is_ok());
    }

    #[test]
    fn follows_a_symlink_to_the_real_path() {
        let base = workspace("symlink");
        let real = base.join("img/real.png");
        write(&real);
        std::os::unix::fs::symlink(&real, base.join("img/link.png")).expect("a symlink");
        let resolved = resolve(&base, "img/link.png").expect("a path");
        assert_eq!(resolved, fs::canonicalize(&real).unwrap());
    }

    #[test]
    fn rejects_a_missing_file() {
        let base = workspace("missing");
        assert!(resolve(&base, "img/gone.png").is_err());
    }

    #[test]
    fn rejects_an_extension_that_is_not_an_image() {
        let base = workspace("extension");
        write(&base.join("img/notes.txt"));
        assert!(resolve(&base, "img/notes.txt").is_err());
    }

    #[test]
    fn rejects_a_directory() {
        let base = workspace("directory");
        fs::create_dir_all(base.join("img/inner.png")).expect("a directory");
        assert!(resolve(&base, "img/inner.png").is_err());
    }

    #[test]
    fn accepts_an_uppercase_extension() {
        let base = workspace("uppercase");
        write(&base.join("img/a.PNG"));
        assert!(resolve(&base, "img/a.PNG").is_ok());
    }
}
