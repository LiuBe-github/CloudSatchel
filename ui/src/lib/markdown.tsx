// 轻量 Markdown 渲染器（零第三方依赖，离线可构建）
//
// AI 回复按 Markdown 语法渲染为 React 元素：全程构造 React 节点，
// 不使用 dangerouslySetInnerHTML，无 XSS 面。
//
// 支持：标题 / 分割线 / 引用 / 围栏代码块（带复制按钮）/ 有序·无序列表 /
//       表格 / 链接（http/https 用系统默认浏览器打开）/ 图片 / 行内代码 /
//       粗体 / 斜体 / 删除线 / 自动链接 / 换行（单换行渲染为 <br>）
import { createElement, useMemo, useState } from "react";
import type { MouseEvent, ReactNode } from "react";
import { openUrl } from "./bridge";

// ---------------------------------------------------------------------------
// 行内元素
// ---------------------------------------------------------------------------

/** 按正则切分文本，命中片段用 wrap 包裹，其余交给 fallback 继续解析 */
function splitApply(
  text: string,
  re: RegExp,
  keyBase: string,
  wrap: (inner: string, key: string) => ReactNode,
  fallback: (rest: string, key: string) => ReactNode[],
): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let idx = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) out.push(...fallback(text.slice(last, m.index), `${keyBase}-t${idx}`));
    out.push(wrap(m[1], `${keyBase}-w${idx}`));
    last = m.index + m[0].length;
    idx += 1;
  }
  if (last < text.length) out.push(...fallback(text.slice(last), `${keyBase}-t${idx}`));
  return out;
}

const plain = (text: string, key: string): ReactNode[] => [<span key={key}>{text}</span>];

const renderEm = (text: string, key: string): ReactNode[] =>
  splitApply(text, /\*([^*\n]+)\*/g, key, (t, k) => <em key={k}>{t}</em>, plain);

const renderStrike = (text: string, key: string): ReactNode[] =>
  splitApply(text, /~~([^~\n]+)~~/g, key, (t, k) => <del key={k}>{t}</del>, renderEm);

const renderBold = (text: string, key: string): ReactNode[] =>
  splitApply(
    text,
    /(\*\*[^*\n]+\*\*|__[^_\n]+__)/g,
    key,
    (t, k) => <strong key={k}>{renderStrike(t.slice(2, -2), k)}</strong>,
    renderStrike,
  );

function onLinkClick(e: MouseEvent<HTMLAnchorElement>) {
  e.preventDefault();
  const href = e.currentTarget.getAttribute("href") ?? "";
  if (/^https?:\/\//i.test(href)) {
    void openUrl(href);
  }
}

/** 链接 / 图片 / 自动链接 */
function renderLinks(text: string, keyBase: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const re = /!\[([^\]]*)\]\(([^)\s]+)\)|\[([^\]]*)\]\(([^)\s]+)\)|https?:\/\/[^\s<>"')\]]+/g;
  let last = 0;
  let idx = 0;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m.index > last) nodes.push(...renderBold(text.slice(last, m.index), `${keyBase}-x${idx}`));
    if (m[2] !== undefined) {
      // 图片 ![alt](url)
      nodes.push(
        <img
          key={`${keyBase}-img${idx}`}
          className="md-img"
          src={m[2]}
          alt={m[1] ?? ""}
          loading="lazy"
          onError={(e) => {
            e.currentTarget.style.display = "none";
          }}
        />,
      );
    } else {
      // 链接 [label](url) 或自动链接
      const href = m[4] ?? m[0];
      nodes.push(
        <a key={`${keyBase}-a${idx}`} className="md-link" href={href} onClick={onLinkClick} title={href}>
          {m[3] !== undefined ? renderInline(m[3], `${keyBase}-al${idx}`) : href}
        </a>,
      );
    }
    last = m.index + m[0].length;
    idx += 1;
  }
  if (last < text.length) nodes.push(...renderBold(text.slice(last), `${keyBase}-x${idx}`));
  return nodes;
}

/** 行内渲染：行内代码 → 链接/图片 → 粗体 → 删除线 → 斜体 */
function renderInline(text: string, keyBase: string): ReactNode[] {
  const nodes: ReactNode[] = [];
  const parts = text.split(/(`[^`\n]+`)/g);
  parts.forEach((part, i) => {
    const key = `${keyBase}-c${i}`;
    if (/^`[^`\n]+`$/.test(part)) {
      nodes.push(
        <code key={key} className="md-inline-code">
          {part.slice(1, -1)}
        </code>,
      );
      return;
    }
    nodes.push(...renderLinks(part, key));
  });
  return nodes;
}

// ---------------------------------------------------------------------------
// 块级元素
// ---------------------------------------------------------------------------

function CodeBlock({ code, lang }: { code: string; lang: string }) {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    const done = () => {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    };
    void navigator.clipboard.writeText(code).then(done, () => {
      // 剪贴板 API 不可用时降级为选中复制
      try {
        const ta = document.createElement("textarea");
        ta.value = code;
        ta.style.position = "fixed";
        ta.style.opacity = "0";
        document.body.appendChild(ta);
        ta.select();
        document.execCommand("copy");
        document.body.removeChild(ta);
      } catch {
        /* 忽略复制失败 */
      }
      done();
    });
  };
  return (
    <div className="md-code">
      <div className="md-code-head">
        <span className="md-code-lang">{lang || "code"}</span>
        <button className="md-code-copy" onClick={copy} type="button">
          {copied ? "已复制 ✓" : "复制"}
        </button>
      </div>
      <pre className="md-code-pre">
        <code>{code}</code>
      </pre>
    </div>
  );
}

function splitRow(line: string): string[] {
  let s = line.trim();
  if (s.startsWith("|")) s = s.slice(1);
  if (s.endsWith("|")) s = s.slice(0, -1);
  return s.split("|").map((c) => c.trim());
}

function renderTable(headerLine: string, sepLine: string, rows: string[][], key: number): ReactNode {
  const headers = splitRow(headerLine);
  const aligns = splitRow(sepLine).map((c) => {
    const l = c.startsWith(":");
    const r = c.endsWith(":");
    if (l && r) return "center" as const;
    if (r) return "right" as const;
    return "left" as const;
  });
  const align = (i: number) => aligns[Math.min(i, aligns.length - 1)] ?? "left";
  return (
    <div key={key} className="md-table-wrap">
      <table className="md-table">
        <thead>
          <tr>
            {headers.map((h, i) => (
              <th key={i} style={{ textAlign: align(i) }}>
                {renderInline(h, `th${key}-${i}`)}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, ri) => (
            <tr key={ri}>
              {row.map((cell, ci) => (
                <td key={ci} style={{ textAlign: align(ci) }}>
                  {renderInline(cell, `td${key}-${ri}-${ci}`)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

const LIST_ITEM_RE = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/;

function parseList(
  lines: string[],
  start: number,
  keyRef: { k: number },
): { node: ReactNode; next: number } {
  const first = LIST_ITEM_RE.exec(lines[start]);
  if (!first) return { node: null, next: start };
  const baseIndent = first[1].length;
  const ordered = /\d+[.)]/.test(first[2]);
  const items: ReactNode[] = [];
  let i = start;
  while (i < lines.length) {
    const m = LIST_ITEM_RE.exec(lines[i]);
    if (!m || m[1].length < baseIndent) break;
    if (m[1].length === baseIndent && (/\d+[.)]/.test(m[2]) !== ordered)) break;
    if (m[1].length > baseIndent) {
      // 更深缩进的嵌套列表
      const res = parseList(lines, i, keyRef);
      if (res.node) items.push(res.node);
      i = res.next;
      continue;
    }
    // 同级新条目：收集条目内容与嵌套列表
    const content: string[] = [m[3]];
    const nested: ReactNode[] = [];
    i += 1;
    while (i < lines.length && lines[i].trim() !== "") {
      const sub = LIST_ITEM_RE.exec(lines[i]);
      if (sub) {
        if (sub[1].length > baseIndent) {
          const res = parseList(lines, i, keyRef);
          if (res.node) nested.push(res.node);
          i = res.next;
          continue;
        }
        break; // 同级或更浅：结束本条目
      }
      content.push(lines[i]);
      i += 1;
    }
    const itemKey = keyRef.k++;
    items.push(
      <li key={itemKey}>
        {content.map((line, j) => (
          <span key={j}>
            {j > 0 && <br />}
            {renderInline(line, `li${itemKey}-${j}`)}
          </span>
        ))}
        {nested}
      </li>,
    );
    // 跳过条目间空行
    while (i < lines.length && lines[i].trim() === "") i += 1;
    if (i >= lines.length) break;
    const nxt = LIST_ITEM_RE.exec(lines[i]);
    if (!nxt || nxt[1].length < baseIndent) break;
    if (nxt[1].length === baseIndent && /\d+[.)]/.test(nxt[2]) !== ordered) break;
  }
  return {
    node:
      items.length > 0 ? (
        ordered ? (
          <ol key={keyRef.k++} className="md-ol">
            {items}
          </ol>
        ) : (
          <ul key={keyRef.k++} className="md-ul">
            {items}
          </ul>
        )
      ) : null,
    next: i,
  };
}

const HEADINGS = { 1: "h1", 2: "h2", 3: "h3", 4: "h4", 5: "h5", 6: "h6" } as const;

function parseBlocks(content: string): ReactNode[] {
  const lines = content.replace(/\r\n?/g, "\n").split("\n");
  const out: ReactNode[] = [];
  const keyRef = { k: 0 };
  const key = () => keyRef.k++;
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];

    if (line.trim() === "") {
      i += 1;
      continue;
    }

    // 围栏代码块
    const fence = /^\s{0,3}(```|~~~)\s*(.*)$/.exec(line);
    if (fence) {
      const lang = fence[2].trim();
      const buf: string[] = [];
      i += 1;
      while (i < lines.length && !/^\s{0,3}(```|~~~)\s*$/.test(lines[i])) {
        buf.push(lines[i]);
        i += 1;
      }
      i += 1; // 跳过闭合围栏
      out.push(<CodeBlock key={key()} code={buf.join("\n")} lang={lang} />);
      continue;
    }

    // 标题
    const heading = /^(#{1,6})\s+(.+)$/.exec(line);
    if (heading) {
      const level = heading[1].length as 1 | 2 | 3 | 4 | 5 | 6;
      out.push(
        createElement(
          HEADINGS[level],
          { key: key(), className: "md-h" },
          ...renderInline(heading[2], `h${keyRef.k}`),
        ),
      );
      i += 1;
      continue;
    }

    // 分割线
    if (/^\s{0,3}(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
      out.push(<hr key={key()} className="md-hr" />);
      i += 1;
      continue;
    }

    // 引用
    if (/^\s{0,3}>/.test(line)) {
      const buf: string[] = [];
      while (i < lines.length && /^\s{0,3}>/.test(lines[i])) {
        buf.push(lines[i].replace(/^\s{0,3}>\s?/, ""));
        i += 1;
      }
      out.push(
        <blockquote key={key()} className="md-quote">
          {parseBlocks(buf.join("\n"))}
        </blockquote>,
      );
      continue;
    }

    // 表格：本行含 | 且下一行是分隔行
    if (line.includes("|") && i + 1 < lines.length) {
      const sep = lines[i + 1];
      if (/^\s*\|?[\s:|-]+\|[\s:|-]*$/.test(sep) && sep.includes("-")) {
        const rows: string[][] = [];
        let j = i + 2;
        while (j < lines.length && lines[j].trim() !== "" && lines[j].includes("|")) {
          rows.push(splitRow(lines[j]));
          j += 1;
        }
        out.push(renderTable(line, sep, rows, key()));
        i = j;
        continue;
      }
    }

    // 列表
    if (LIST_ITEM_RE.test(line)) {
      const res = parseList(lines, i, keyRef);
      if (res.node) out.push(res.node);
      i = res.next;
      continue;
    }

    // 段落（连续行，遇空行/代码围栏/标题/分割线/引用/列表/表格头结束）
    const buf: string[] = [];
    while (i < lines.length && lines[i].trim() !== "") {
      const cur = lines[i];
      if (/^\s{0,3}(```|~~~)/.test(cur)) break;
      if (/^(#{1,6})\s+/.test(cur)) break;
      if (/^\s{0,3}(-{3,}|\*{3,}|_{3,})\s*$/.test(cur)) break;
      if (/^\s{0,3}>/.test(cur)) break;
      if (LIST_ITEM_RE.test(cur)) break;
      if (cur.includes("|") && i + 1 < lines.length) {
        const s2 = lines[i + 1];
        if (/^\s*\|?[\s:|-]+\|[\s:|-]*$/.test(s2) && s2.includes("-")) break;
      }
      buf.push(cur);
      i += 1;
    }
    const pKey = key();
    out.push(
      <p key={pKey} className="md-p">
        {buf.map((l, j) => (
          <span key={j}>
            {j > 0 && <br />}
            {renderInline(l, `p${pKey}-${j}`)}
          </span>
        ))}
      </p>,
    );
  }
  return out;
}

// ---------------------------------------------------------------------------
// 组件
// ---------------------------------------------------------------------------

/** AI 回复的 Markdown 渲染组件（流式输出期间每个增量都会重新解析） */
export function Markdown({ content }: { content: string }) {
  const nodes = useMemo(() => parseBlocks(content), [content]);
  return <div className="md">{nodes}</div>;
}
