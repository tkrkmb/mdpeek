import { describe, expect, test } from "vitest";

import { compileVimPattern, hasUppercase, vimMatches } from "./vimregex";

/** 検索語で text を探し、一致した文字列を並べる。検索語が正しくなければ null */
function find(pattern: string, text: string): string[] | null {
  const regex = compileVimPattern(pattern);
  if (regex === null) {
    return null;
  }
  return vimMatches(text, regex).map((range) => text.slice(range.start, range.end));
}

describe("compileVimPattern", () => {
  test("keeps plain text and treats magic characters as Vim does", () => {
    expect(find("a.b", "a.b axb")).toEqual(["a.b", "axb"]);
    expect(find("a\\.b", "a.b axb")).toEqual(["a.b"]);
    expect(find("(c)", "(c) c")).toEqual(["(c)"]);
    expect(find("a/b", "a/b")).toEqual(["a/b"]);
  });

  test("matches whole words with \\< and \\>", () => {
    expect(find("\\<page\\>", "page pages webpage page.")).toEqual(["page", "page"]);
  });

  test("supports alternatives, groups and repeats", () => {
    expect(find("foo\\|bar", "foo bar baz")).toEqual(["foo", "bar"]);
    expect(find("\\v(foo|bar)+", "foobarfoo x bar")).toEqual(["foobarfoo", "bar"]);
    expect(find("\\%(ab\\)\\+", "ababx")).toEqual(["abab"]);
    expect(find("colou\\=r", "color colour colouur")).toEqual(["color", "colour"]);
    expect(find("colou\\?r", "color colour")).toEqual(["color", "colour"]);
    expect(find("a\\{2,3}", "a aa aaaa")).toEqual(["aa", "aaa"]);
    expect(find("a\\{2}", "aaaaa")).toEqual(["aa", "aa"]);
    expect(find("a\\{,2}b", "aaab")).toEqual(["aab"]);
    expect(find("a\\{-1,}", "aaa")).toEqual(["a", "a", "a"]);
    expect(find("x.\\{-}y", "x1y2y")).toEqual(["x1y"]);
  });

  test("anchors ^ and $ to the start and end of each line", () => {
    expect(find("^#", "# a\nb #\n# c")).toEqual(["#", "#"]);
    expect(find("end$", "end x\nthe end")).toEqual(["end"]);
    expect(find("a^b", "a^b")).toEqual(["a^b"]);
    expect(find("a$b", "a$b")).toEqual(["a$b"]);
  });

  test("narrows the match with \\zs and \\ze", () => {
    expect(find("foo\\zsbar", "foobar bar")).toEqual(["bar"]);
    expect(find("foo\\zebar", "foobar foo")).toEqual(["foo"]);
  });

  test("supports character classes and collections", () => {
    expect(find("\\d\\+", "a12 b3")).toEqual(["12", "3"]);
    expect(find("\\u\\l\\+", "Hello world Foo")).toEqual(["Hello", "Foo"]);
    expect(find("\\S\\+", "ab  cd")).toEqual(["ab", "cd"]);
    expect(find("[a-c]\\+", "abcd cab")).toEqual(["abc", "cab"]);
    expect(find("[^ ]\\+", "ab cd")).toEqual(["ab", "cd"]);
    expect(find("[[:digit:]]\\+", "x42")).toEqual(["42"]);
    expect(find("[]x]", "a]x")).toEqual(["]", "x"]);
  });

  test("switches between very magic, magic, nomagic and very nomagic", () => {
    expect(find("\\va+", "aaa")).toEqual(["aaa"]);
    expect(find("\\Ma*", "a* aa")).toEqual(["a*"]);
    expect(find("\\Va.b", "a.b axb")).toEqual(["a.b"]);
    expect(find("\\Va\\.b", "a.b axb")).toEqual(["a.b", "axb"]);
  });

  test("uses \\c and \\C, or smartcase without them", () => {
    expect(find("\\cREADME", "readme README")).toEqual(["readme", "README"]);
    expect(find("\\Creadme", "readme README")).toEqual(["readme"]);
    expect(find("readme", "readme README")).toEqual(["readme", "README"]);
    expect(find("README", "readme README")).toEqual(["README"]);
    // \S や \%( の後ろの文字は、大文字として数えない
    expect(hasUppercase("\\Sfoo")).toBe(false);
    expect(hasUppercase("\\%Vfoo")).toBe(false);
    expect(find("\\Sa", "Xa xa")).toEqual(["Xa", "xa"]);
  });

  test("keeps \\u, \\l and [:upper:] case-sensitive when ignoring case, as Vim does", () => {
    // Neovim で ignorecase のときに matchbufline で確かめた結果と同じ
    expect(find("\\c\\u\\l\\+", "Hello world Foo")).toEqual(["Hello", "Foo"]);
    expect(find("\\c[A-Z]\\+", "Hello world Foo")).toEqual(["Hello", "world", "Foo"]);
    expect(find("\\c[[:upper:]]\\+", "Hello world Foo")).toEqual(["H", "F"]);
    expect(find("\\c[^a-z ]", "aB1")).toEqual(["1"]);
  });

  test("rejects invalid or unsupported patterns", () => {
    expect(compileVimPattern("\\(")).toBeNull();
    expect(compileVimPattern("a\\)")).toBeNull();
    expect(compileVimPattern("\\%Vfoo")).toBeNull();
    expect(compileVimPattern("\\%23l")).toBeNull();
    expect(compileVimPattern("foo\\@=")).toBeNull();
    expect(compileVimPattern("~")).toBeNull();
    expect(compileVimPattern("\\1")).toBeNull();
    expect(compileVimPattern("\\n")).toBeNull();
    expect(compileVimPattern("a\\_sb")).toBeNull();
    expect(compileVimPattern("a\\{x}")).toBeNull();
    expect(compileVimPattern("a\\")).toBeNull();
    expect(compileVimPattern("\\+")).toBeNull();
  });

  test("treats a leading * as a literal star", () => {
    expect(find("*a", "*a a")).toEqual(["*a"]);
  });

  test("skips empty matches", () => {
    expect(find("^", "a\nb")).toEqual([]);
    expect(find("x*", "axxb")).toEqual(["xx"]);
  });
});
