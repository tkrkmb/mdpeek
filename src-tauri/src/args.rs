use std::path::PathBuf;

pub struct Args {
    pub socket: String,
    pub token: String,
}

/// 起動引数から決まる、動く相手
pub enum Mode {
    Nvim(Args),
    File { path: PathBuf, foreground: bool },
    /// `--version`：版を表示して終わる
    Version,
}

const USAGE: &str =
    "usage: mdsight --nvim <socket> --token <token> | mdsight [--foreground] <file> | mdsight --version";

/// `--nvim <socket> --token <token>` ならNeovim連携モード、
/// 単一の位置引数（ファイルパス、`--foreground` を付けてもよい）ならスタンドアローンモードにする。
/// `--version` だけなら、版を表示して終わる。
pub fn parse_args(argv: impl Iterator<Item = String>) -> Result<Mode, String> {
    let mut socket = None;
    let mut token = None;
    let mut foreground = false;
    let mut version = false;
    let mut positional = Vec::new();
    let mut argv = argv;
    while let Some(arg) = argv.next() {
        match arg.as_str() {
            "--nvim" => socket = argv.next(),
            "--token" => token = argv.next(),
            "--foreground" => foreground = true,
            "--version" => version = true,
            other if other.starts_with("--") => return Err(format!("unknown argument: {other}")),
            other => positional.push(other.to_string()),
        }
    }
    if version {
        // `--version` は、他の引数と組み合わせない
        return if socket.is_none() && token.is_none() && !foreground && positional.is_empty() {
            Ok(Mode::Version)
        } else {
            Err(USAGE.to_string())
        };
    }
    match (socket, token, positional.as_slice()) {
        (Some(socket), Some(token), []) if !foreground => Ok(Mode::Nvim(Args { socket, token })),
        (None, None, [path]) => Ok(Mode::File {
            path: PathBuf::from(path),
            foreground,
        }),
        _ => Err(USAGE.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_args, Mode};

    #[test]
    fn parses_the_expected_arguments() {
        let mode = parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc"]
                .into_iter()
                .map(String::from),
        )
        .expect("a mode");
        match mode {
            Mode::Nvim(args) => {
                assert_eq!(args.socket, "/tmp/nvim.sock");
                assert_eq!(args.token, "abc");
            }
            _ => panic!("expected nvim mode"),
        }
    }

    #[test]
    fn rejects_missing_arguments() {
        assert!(parse_args(["--nvim", "/tmp/nvim.sock"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn parses_a_single_file_argument() {
        let mode = parse_args(["note.md"].into_iter().map(String::from)).expect("a mode");
        match mode {
            Mode::File { path, foreground } => {
                assert_eq!(path, std::path::PathBuf::from("note.md"));
                assert!(!foreground, "standalone mode runs in the background by default");
            }
            _ => panic!("expected file mode"),
        }
    }

    #[test]
    fn parses_the_foreground_flag() {
        let mode = parse_args(["--foreground", "note.md"].into_iter().map(String::from))
            .expect("a mode");
        match mode {
            Mode::File { foreground, .. } => assert!(foreground),
            _ => panic!("expected file mode"),
        }
    }

    #[test]
    fn rejects_the_foreground_flag_for_neovim() {
        assert!(parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc", "--foreground"]
                .into_iter()
                .map(String::from)
        )
        .is_err());
    }

    #[test]
    fn rejects_the_foreground_flag_alone() {
        assert!(parse_args(["--foreground"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn rejects_mixed_arguments() {
        assert!(parse_args(
            ["--nvim", "/tmp/nvim.sock", "--token", "abc", "note.md"]
                .into_iter()
                .map(String::from)
        )
        .is_err());
    }

    #[test]
    fn parses_the_version_flag() {
        let mode = parse_args(["--version"].into_iter().map(String::from)).expect("a mode");
        assert!(matches!(mode, Mode::Version));
    }

    #[test]
    fn rejects_the_version_flag_with_other_arguments() {
        assert!(parse_args(["--version", "note.md"].into_iter().map(String::from)).is_err());
        assert!(parse_args(["--foreground", "--version"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn rejects_two_file_arguments() {
        assert!(parse_args(["a.md", "b.md"].into_iter().map(String::from)).is_err());
    }
}
