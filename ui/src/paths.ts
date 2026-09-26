const SCHEME = /^[a-z][a-z0-9+.-]*:/i;

/** 文書のディレクトリを基準に解決できる相対パスか（空、スキーム付き、`/` で始まる絶対パスと `//` は違う） */
export function isRelativePath(path: string): boolean {
  return path !== "" && !SCHEME.test(path) && !path.startsWith("/");
}
