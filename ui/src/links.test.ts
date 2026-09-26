import { describe, expect, test } from "vitest";

import { isRelativeMarkdownLink } from "./links";

describe("isRelativeMarkdownLink", () => {
  test("accepts a relative .md link", () => {
    expect(isRelativeMarkdownLink("other.md")).toBe(true);
  });

  test("accepts the .markdown extension", () => {
    expect(isRelativeMarkdownLink("other.markdown")).toBe(true);
  });

  test("accepts a heading fragment after the file", () => {
    expect(isRelativeMarkdownLink("other.md#section")).toBe(true);
  });

  test("accepts a parent-directory link", () => {
    expect(isRelativeMarkdownLink("../notes/other.md")).toBe(true);
  });

  test("is case-insensitive about the extension", () => {
    expect(isRelativeMarkdownLink("OTHER.MD")).toBe(true);
  });

  test("rejects a plain in-page fragment", () => {
    expect(isRelativeMarkdownLink("#section")).toBe(false);
  });

  test("rejects an http link", () => {
    expect(isRelativeMarkdownLink("https://example.com/other.md")).toBe(false);
  });

  test("rejects a mailto link", () => {
    expect(isRelativeMarkdownLink("mailto:a@example.com")).toBe(false);
  });

  test("rejects a non-markdown extension", () => {
    expect(isRelativeMarkdownLink("image.png")).toBe(false);
  });

  test("rejects an absolute path", () => {
    expect(isRelativeMarkdownLink("/notes/other.md")).toBe(false);
  });

  test("rejects a network path", () => {
    expect(isRelativeMarkdownLink("//host/other.md")).toBe(false);
  });

  test("rejects an empty href", () => {
    expect(isRelativeMarkdownLink("")).toBe(false);
  });
});
