import type { Range } from "./search";

/**
 * Neovimの検索の書き方（Vimの正規表現）を、JavaScript の正規表現に変換する。
 * 対応する書き方は AGENTS.md の「ページ内検索」のとおり。それ以外は null を返す。
 *
 * 大文字と小文字を区別しないときも `i` フラグは使わず、文字や範囲に両方の形を並べる。
 * Vimでは \u、\l、[:upper:] などが、大文字と小文字を区別したままだから
 */

type Magic = "v" | "m" | "M" | "V";

/** 書き方の切り替えによって意味が変わる記号 */
const OPERATORS = "()|+=?{<>%.*[~^$@&";

/** `\` を付けずに書いたときに、記号として働くもの（それ以外は `\` を付けたときに働く） */
const PLAIN: Record<Magic, string> = {
  v: OPERATORS,
  m: ".*[~^$",
  M: "^$",
  V: "",
};

/** 単語の文字（`\<` と `\>` の境界に使う） */
const WORD = "[\\p{L}\\p{N}_]";

/** 文字クラス。否定のクラスは、Vimと同じく改行に一致させない */
const CLASSES: Record<string, string> = {
  s: "[ \\t]",
  S: "[^ \\t\\n]",
  d: "[0-9]",
  D: "[^0-9\\n]",
  w: "[0-9A-Za-z_]",
  W: "[^0-9A-Za-z_\\n]",
  a: "[A-Za-z]",
  A: "[^A-Za-z\\n]",
  l: "[a-z]",
  L: "[^a-z\\n]",
  u: "[A-Z]",
  U: "[^A-Z\\n]",
  x: "[0-9A-Fa-f]",
  X: "[^0-9A-Fa-f\\n]",
};

/** `[[:alpha:]]` などの名前付きのクラス */
const NAMED: Record<string, string> = {
  alpha: "A-Za-z",
  digit: "0-9",
  alnum: "0-9A-Za-z",
  lower: "a-z",
  upper: "A-Z",
  xdigit: "0-9A-Fa-f",
  space: " \\t\\n\\r\\f\\v",
  blank: " \\t",
  punct: "!-\\/:-@\\[-`{-~",
};

/** 集合の中で、`\` を付けると別の文字を表すもの */
const COLLECTION_ESCAPES: Record<string, string> = { e: "\x1b", t: "\t", r: "\r", n: "\n" };

/** 大文字と小文字で形が変わる1文字なら、もう一方の形を返す */
function otherCase(char: string): string | null {
  const lower = char.toLowerCase();
  const upper = char.toUpperCase();
  if (lower === upper || [...lower].length !== 1 || [...upper].length !== 1) {
    return null;
  }
  return char === lower ? upper : lower;
}

/** 1文字を表す。大文字と小文字を区別しないときは、両方の形を並べる */
function literal(char: string, ignore: boolean): string {
  const other = ignore ? otherCase(char) : null;
  if (other !== null) {
    return `[${char}${other}]`;
  }
  return /[\\^$.*+?()[\]{}|/]/.test(char) ? `\\${char}` : char;
}

/** 集合の中の1文字を表す */
function member(char: string): string {
  if (char === "\n") {
    return "\\n";
  }
  if (char === "\t") {
    return "\\t";
  }
  if (char === "\r") {
    return "\\r";
  }
  if (char === "\x1b") {
    return "\\x1b";
  }
  return /[\\\][^-]/.test(char) ? `\\${char}` : char;
}

/** 大文字と小文字を区別しないときに、集合へ足す、もう一方の形の文字や範囲（範囲はASCIIの英字だけ） */
function otherCaseMember(first: string, last: string | null): string {
  if (last === null) {
    const other = otherCase(first);
    return other === null ? "" : member(other);
  }
  const within = (low: string, high: string) => first >= low && first <= high && last >= low && last <= high;
  if (within("a", "z") || within("A", "Z")) {
    return `${otherCase(first)}-${otherCase(last)}`;
  }
  return "";
}

/** 検索語に大文字が含まれるか（`\` に続く文字は数えない。`\%x`、`\_x`、`\zx` は2文字とも数えない） */
export function hasUppercase(pattern: string): boolean {
  return /\p{Lu}/u.test(pattern.replace(/\\[%_z]./gu, "").replace(/\\./gu, ""));
}

/** `\c` か `\C` が書かれていれば、それに従う（両方あれば `\c`）。どちらもなければ smartcase */
function ignoresCase(pattern: string): boolean {
  let lower = false;
  let upper = false;
  for (let index = 0; index < pattern.length; index += 1) {
    if (pattern[index] === "\\") {
      lower ||= pattern[index + 1] === "c";
      upper ||= pattern[index + 1] === "C";
      index += 1;
    }
  }
  if (lower) {
    return true;
  }
  return upper ? false : !hasUppercase(pattern);
}

/** `[` の後ろから、閉じる `]` までを読む。閉じていなければ null（`[` は文字として扱う） */
function collection(
  pattern: string,
  from: number,
  ignore: boolean,
): { source: string; end: number } | null {
  let index = from;
  let negate = false;
  if (pattern[index] === "^") {
    negate = true;
    index += 1;
  }
  // 集合の中の1文字を読む。[\xyz] のような、意味のない \ は、\ そのものとして扱う
  const readChar = (at: number): { char: string; end: number } => {
    if (pattern[at] === "\\" && at + 1 < pattern.length) {
      const next = pattern[at + 1];
      if (next in COLLECTION_ESCAPES) {
        return { char: COLLECTION_ESCAPES[next], end: at + 2 };
      }
      if ("\\]^-".includes(next)) {
        return { char: next, end: at + 2 };
      }
    }
    return { char: pattern[at], end: at + 1 };
  };

  let body = "";
  // 先頭の ] は文字として扱う
  let first = true;
  while (index < pattern.length && (pattern[index] !== "]" || first)) {
    first = false;
    if (pattern[index] === "[" && pattern[index + 1] === ":") {
      const close = pattern.indexOf(":]", index + 2);
      const named = close === -1 ? undefined : NAMED[pattern.slice(index + 2, close)];
      if (named === undefined) {
        return null;
      }
      body += named;
      index = close + 2;
      continue;
    }
    const start = readChar(index);
    index = start.end;
    // a-z のような範囲（- の後ろが ] なら、- は文字）
    if (pattern[index] === "-" && index + 1 < pattern.length && pattern[index + 1] !== "]") {
      const end = readChar(index + 1);
      index = end.end;
      body += `${member(start.char)}-${member(end.char)}`;
      if (ignore) {
        body += otherCaseMember(start.char, end.char);
      }
      continue;
    }
    body += member(start.char);
    if (ignore) {
      body += otherCaseMember(start.char, null);
    }
  }
  if (index >= pattern.length) {
    return null;
  }
  return { source: negate ? `[^${body}\\n]` : `[${body}]`, end: index + 1 };
}

/** `\{` の後ろから、閉じる `}`（`\}` でもよい）までを読み、JavaScript の繰り返しにする */
function braces(pattern: string, from: number): { source: string; end: number } | null {
  let index = from;
  let body = "";
  while (index < pattern.length && pattern[index] !== "}") {
    if (pattern[index] === "\\" && pattern[index + 1] === "}") {
      break;
    }
    body += pattern[index];
    index += 1;
  }
  if (index >= pattern.length) {
    return null;
  }
  const end = pattern[index] === "\\" ? index + 2 : index + 1;
  const match = /^(-?)(\d*)(?:(,)(\d*))?$/.exec(body);
  if (match === null) {
    return null;
  }
  const [, lazy, low, comma, high] = match;
  let source: string;
  if (comma === undefined) {
    source = low === "" ? "*" : `{${low}}`;
  } else if (low !== "" && high !== "" && Number(low) > Number(high)) {
    // Vimは大小が逆でも受け付ける
    source = `{${high},${low}}`;
  } else {
    source = `{${low === "" ? "0" : low},${high}}`;
  }
  return { source: lazy === "-" ? `${source}?` : source, end };
}

/**
 * Vimの検索語を、JavaScript の正規表現に変換する。対応しない書き方や、正しくない検索語なら null。
 * `\zs` と `\ze` は、空の名前付きグループ（zs0、ze0、…）にして、一致の範囲を決めるのに使う
 */
export function compileVimPattern(pattern: string): RegExp | null {
  const ignore = ignoresCase(pattern);
  let magic: Magic = "m";
  let source = "";
  let starts = 0;
  let ends = 0;
  // 直前が繰り返せるもの（文字やグループ）か
  let repeatable = false;
  // 検索語や選択肢の先頭か（^ が行頭の意味になる）
  let branchStart = true;
  let index = 0;

  const token = (at: number): { char: string; escaped: boolean; end: number } | null => {
    if (pattern[at] !== "\\") {
      return { char: pattern[at], escaped: false, end: at + 1 };
    }
    return at + 1 < pattern.length ? { char: pattern[at + 1], escaped: true, end: at + 2 } : null;
  };
  const isOperator = (char: string, escaped: boolean): boolean =>
    OPERATORS.includes(char) && PLAIN[magic].includes(char) !== escaped;

  while (index < pattern.length) {
    const current = token(index);
    if (current === null) {
      // 末尾の \ だけ
      return null;
    }
    index = current.end;
    const { char, escaped } = current;

    if (isOperator(char, escaped)) {
      switch (char) {
        case "^":
          source += branchStart ? "^" : "\\^";
          repeatable = !branchStart;
          branchStart = false;
          continue;
        case "$": {
          const next = index < pattern.length ? token(index) : null;
          const atEnd =
            index >= pattern.length ||
            (next !== null && (next.char === "|" || next.char === ")") && isOperator(next.char, next.escaped));
          source += atEnd ? "$" : "\\$";
          repeatable = !atEnd;
          branchStart = false;
          continue;
        }
        case ".":
          source += ".";
          repeatable = true;
          break;
        case "*":
          // 先頭の * は文字として扱う
          source += repeatable ? "*" : "\\*";
          repeatable = !repeatable;
          break;
        case "+":
        case "=":
        case "?":
          if (!repeatable) {
            return null;
          }
          source += char === "+" ? "+" : "?";
          repeatable = false;
          break;
        case "{": {
          if (!repeatable) {
            return null;
          }
          const read = braces(pattern, index);
          if (read === null) {
            return null;
          }
          source += read.source;
          index = read.end;
          repeatable = false;
          break;
        }
        case "(":
          source += "(";
          repeatable = false;
          branchStart = true;
          continue;
        case ")":
          source += ")";
          repeatable = true;
          branchStart = false;
          continue;
        case "|":
          source += "|";
          repeatable = false;
          branchStart = true;
          continue;
        case "<":
          source += `(?<!${WORD})(?=${WORD})`;
          repeatable = false;
          break;
        case ">":
          source += `(?<=${WORD})(?!${WORD})`;
          repeatable = false;
          break;
        case "%":
          // 対応するのは \%( だけ
          if (pattern[index] !== "(") {
            return null;
          }
          index += 1;
          source += "(?:";
          repeatable = false;
          branchStart = true;
          continue;
        case "[": {
          const read = collection(pattern, index, ignore);
          if (read === null) {
            source += "\\[";
          } else {
            source += read.source;
            index = read.end;
          }
          repeatable = true;
          break;
        }
        default:
          // ~、\@、\& には対応しない
          return null;
      }
      branchStart = false;
      continue;
    }

    if (escaped) {
      if (char in CLASSES) {
        source += CLASSES[char];
        repeatable = true;
        branchStart = false;
        continue;
      }
      if (char === "v" || char === "m" || char === "M" || char === "V") {
        magic = char;
        continue;
      }
      if (char === "c" || char === "C") {
        // 大文字と小文字の扱いは、先に ignoresCase で決めてある
        continue;
      }
      if (char === "z" && (pattern[index] === "s" || pattern[index] === "e")) {
        source += pattern[index] === "s" ? `(?<zs${starts++}>)` : `(?<ze${ends++}>)`;
        index += 1;
        continue;
      }
      // \n、\1 などの英数字と、\_s などの \_ には対応しない
      if (/[0-9A-Za-z_]/.test(char)) {
        return null;
      }
    }

    // 文字として扱う（\\、\/、magic での \. など）
    source += literal(char, ignore);
    repeatable = true;
    branchStart = false;
  }

  try {
    return new RegExp(source, "gmud");
  } catch {
    return null;
  }
}

/** `groups` のうち、名前が `prefix` で始まり、一致に加わったものの位置のうち最後のもの */
function lastGroup(
  groups: Record<string, [number, number] | undefined> | undefined,
  prefix: string,
): number | null {
  if (groups === undefined) {
    return null;
  }
  let found: number | null = null;
  for (let index = 0; `${prefix}${index}` in groups; index += 1) {
    const position = groups[`${prefix}${index}`];
    if (position !== undefined) {
      found = position[0];
    }
  }
  return found;
}

/** 文字列の中の一致の範囲を、前から順に集める。`\zs`／`\ze` があれば範囲を狭め、幅のない一致は除く */
export function vimMatches(text: string, regex: RegExp): Range[] {
  const found: Range[] = [];
  regex.lastIndex = 0;
  for (let match = regex.exec(text); match !== null; match = regex.exec(text)) {
    if (match[0] === "") {
      // 幅のない一致で止まらないように、1文字進める
      const code = text.codePointAt(regex.lastIndex);
      regex.lastIndex += code !== undefined && code > 0xffff ? 2 : 1;
    }
    const groups = match.indices?.groups;
    const start = lastGroup(groups, "zs") ?? match.index;
    const end = lastGroup(groups, "ze") ?? match.index + match[0].length;
    if (end > start) {
      found.push({ start, end });
    }
  }
  return found;
}
