(function () {
  function escapeHtml(text) {
    return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  function highlightEscapesInString(raw) {
    var out = '<span class="tok-string">';
    var i = 0;
    while (i < raw.length) {
      var ch = raw[i];
      
      // Check for the escape character (single backslash)
      if (ch !== "\\") {
        out += escapeHtml(ch);
        i++;
        continue;
      }

      // Handle Escape sequence
      var seq = "\\";
      var next = i + 1 < raw.length ? raw[i + 1] : "";

      if (!next) {
        out += '<span class="tok-escape">' + escapeHtml(seq) + "</span>";
        i++;
        continue;
      }

      // Hex escapes: \xNN
      if (next === "x" && i + 3 < raw.length && /[0-9A-Fa-f]{2}/.test(raw.slice(i + 2, i + 4))) {
        seq = raw.slice(i, i + 4);
        out += '<span class="tok-escape">' + escapeHtml(seq) + "</span>";
        i += 4;
      } 
      // Unicode escapes: \uNNNN
      else if (next === "u" && i + 5 < raw.length && /[0-9A-Fa-f]{4}/.test(raw.slice(i + 2, i + 6))) {
        seq = raw.slice(i, i + 6);
        out += '<span class="tok-escape">' + escapeHtml(seq) + "</span>";
        i += 6;
      } 
      // ES6-style Unicode escapes: \u{...}
      else if (next === "u" && i + 2 < raw.length && raw[i + 2] === "{") {
        var j = i + 3;
        while (j < raw.length && raw[j] !== "}") j++;
        if (j < raw.length && raw[j] === "}") {
          seq = raw.slice(i, j + 1);
          out += '<span class="tok-escape">' + escapeHtml(seq) + "</span>";
          i = j + 1;
        } else {
          // Fallback if closing brace is missing
          out += '<span class="tok-escape">' + escapeHtml(raw.slice(i, i + 2)) + "</span>";
          i += 2;
        }
      } 
      // Standard 2-character escapes: \n, \t, \\, \", etc.
      else {
        seq = raw.slice(i, i + 2);
        out += '<span class="tok-escape">' + escapeHtml(seq) + "</span>";
        i += 2;
      }
    }
    out += "</span>";
    return out;
  }

  function isIdentStart(ch) { return /[A-Za-z_]/.test(ch); }
  function isIdent(ch) { return /[A-Za-z0-9_]/.test(ch); }
  function isDigit(ch) { return /[0-9]/.test(ch); }

  function tokenizeLine(line, state, isErrorBlock) {
    if (isErrorBlock) {
      // 1. Error Header
      var headerMatch = line.match(/^((?:Syntax|Reference|Type|Runtime|Internal)\s+Error:)(.*)/);
      if (headerMatch) {
        var typeKey = headerMatch[1].split(' ')[0].toLowerCase().replace("reference", "ref");
        return `<span class="tok-error-${typeKey}-label">${escapeHtml(headerMatch[1])}</span>` +
               `<span class="tok-error-msg">${escapeHtml(headerMatch[2])}</span>`;
      }

      // 2. Bold Location Line
      if (line.trim().startsWith("|") && line.includes("->")) {
        return `<span class="tok-error-loc">${escapeHtml(line)}</span>`;
      }

      // 3. Meta Lines (expected, found, kind)
      var metaMatch = line.match(/^(\s*)(expected|found|kind)(:\s*)(.*)/);
      if (metaMatch) {
        var valCls = "tok-error-found-val"; // Default to Red
        if (metaMatch[2] === "expected") valCls = "tok-error-expected-val"; // Green
        else if (metaMatch[2] === "kind") valCls = "tok-error-kind-val"; // Also Red
        
        return escapeHtml(metaMatch[1]) + 
               `<span class="tok-error-meta-label">${escapeHtml(metaMatch[2] + metaMatch[3])}</span>` +
               `<span class="${valCls}">${escapeHtml(metaMatch[4])}</span>`;
      }

      // 4. Line Number logic
      var lineNumMatch = line.match(/^(\s*)(\d+)(\s*\|\s*)(.*)/);
      if (lineNumMatch) {
        var num = `<span class="tok-error-line-num">${lineNumMatch[2]}</span>`;
        var rest = tokenizeLine(lineNumMatch[4], { inComment: false }, false);
        return escapeHtml(lineNumMatch[1]) + num + escapeHtml(lineNumMatch[3]) + rest;
      }

      // 5. Pointer Line
      if (line.trim().startsWith("|") && line.includes("^")) {
        var pParts = line.split("^");
        return escapeHtml(pParts[0]) + `<span class="tok-error-pointer">^${escapeHtml(pParts.slice(1).join("^"))}</span>`;
      }
    }

    // --- STANDARD CODE HIGHLIGHTING ---
    var out = "";
    var i = 0;
    while (i < line.length) {
      if (state.inComment) {
        var endIdx = line.indexOf("*/", i);
        if (endIdx !== -1) {
          out += `<span class="tok-comment">${escapeHtml(line.slice(i, endIdx + 2))}</span>`;
          i = endIdx + 2; state.inComment = false; continue;
        } else {
          out += `<span class="tok-comment">${escapeHtml(line.slice(i))}</span>`; break;
        }
      }
      var ch = line[i];
      var next = i + 1 < line.length ? line[i + 1] : "";
      if (ch === "/" && next === "/") { out += `<span class="tok-comment">${escapeHtml(line.slice(i))}</span>`; break; }
      if (ch === "/" && next === "*") { state.inComment = true; continue; }
      if (ch === '"' || ch === "'") {
        var quote = ch; var j = i + 1; var escaped = false;
        while (j < line.length) {
          if (escaped) escaped = false; else if (line[j] === "\\") escaped = true; else if (line[j] === quote) { j++; break; }
          j++;
        }
        out += highlightEscapesInString(line.slice(i, j));
        i = j; state.afterDot = false; continue;
      }
      if (isDigit(ch)) {
        var k = i; while (k < line.length && isDigit(line[k])) k++;
        if (k < line.length && line[k] === ".") { var m = k + 1; while (m < line.length && isDigit(line[m])) m++; k = m; }
        out += `<span class="tok-number">${escapeHtml(line.slice(i, k))}</span>`;
        i = k; state.afterDot = false; continue;
      }
      if (ch === ".") { out += '<span class="tok-operator">.</span>'; state.afterDot = true; i++; continue; }
      if (ch === "@") {
        i++; var bStart = i; while (i < line.length && isIdent(line[i])) i++;
        var ident = line.slice(bStart, i);
        var lookahead = line.slice(i);
        var cls = /^\s*=>/.test(lookahead) ? "tok-builtin" : "tok-property";
        out += `@<span class="${cls}">${escapeHtml(ident)}</span>`;
        state.afterDot = false; continue;
      }
      if (isIdentStart(ch)) {
        var idEnd = i + 1; while (idEnd < line.length && isIdent(line[idEnd])) idEnd++;
        var ident = line.slice(i, idEnd); var rest = line.slice(idEnd);
        var cls = "tok-variable";
        if (state.afterDot) { 
          state.afterDot = false; 
          cls = /^\s*=>/.test(rest) ? "tok-function" : "tok-property"; 
        } 
        else if (/^(if|else|for|while|try|catch|return)$/.test(ident)) cls = "tok-keyword-control";
        else if (/^(use|include|export|from|in|let|const|func)$/.test(ident)) cls = "tok-keyword";
        else if (/^(int|float|bool|string|arr|obj|fn)$/.test(ident)) cls = "tok-type";
        else if (/^(true|false)$/.test(ident)) cls = "tok-boolean";
        else if (/^\s*=>/.test(rest)) cls = "tok-function";
        out += `<span class="${cls}">${escapeHtml(ident)}</span>`;
        i = idEnd;
        if (ident === "func") {
          while (i < line.length && /\s/.test(line[i])) { out += line[i]; i++; }
          var fnStart = i; while (i < line.length && isIdent(line[i])) i++;
          if (i > fnStart) out += `<span class="tok-function">${escapeHtml(line.slice(fnStart, i))}</span>`;
        }
        continue;
      }
      if (/[+\-*/%=<>!|]+/.test(ch)) {
        var opEnd = i + 1; while (opEnd < line.length && /[+\-*/%=<>!|]/.test(line[opEnd])) opEnd++;
        out += `<span class="tok-operator">${escapeHtml(line.slice(i, opEnd))}</span>`;
        i = opEnd; state.afterDot = false; continue;
      }
      if (!/\s/.test(ch)) state.afterDot = false;
      out += escapeHtml(ch); i++;
    }
    return out;
  }

  function highlightCodeElement(codeEl) {
    var isError = codeEl.classList.contains("language-zekken-error");
    if (isError) codeEl.classList.add("zk-code-error");
    var source = codeEl.textContent || "";
    // Preserve raw source for "Run in Demo" even after we replace innerHTML for highlighting.
    codeEl.dataset.zkSource = source;
    var lines = source.split("\n");
    var html = [];
    var state = { inComment: false, afterDot: false };
    for (var i = 0; i < lines.length; i++) {
      html.push(tokenizeLine(lines[i], state, isError));
    }
    codeEl.classList.add("zk-code");
    codeEl.innerHTML = html.join("\n");
  }

  function injectRunButtons() {
    var blocks = document.querySelectorAll("pre code.language-zekken");
    blocks.forEach(function (codeEl) {
      var pre = codeEl.closest("pre");
      if (!pre) return;
      if (pre.querySelector(".zk-run-btn")) return;

      pre.classList.add("zk-codewrap");

      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "zk-run-btn";
      btn.textContent = "Run in Demo";
      btn.title = "Open the demo with this snippet and run it";

      btn.addEventListener("click", function (e) {
        e.preventDefault();
        e.stopPropagation();
        // Reset visual state on the docs page (hover/focus can look "stuck" after opening a new tab).
        try { btn.blur(); } catch (_) {}
        try {
          pre.classList.add("zk-resetting");
          window.setTimeout(function () { pre.classList.remove("zk-resetting"); }, 250);
        } catch (_) {}
        try {
          var src = (codeEl.dataset.zkSource || "").trim();
          // Avoid sending empty blocks to the demo.
          if (!src) return;
          localStorage.setItem("zekken.demo.code", src);
        } catch (_) {}

        // Docs pages live under website/Docs/, demo is website/Demo/demo.html
        window.open("../Demo/demo.html?from=docs&autorun=1", "_blank", "noopener");
      });

      pre.appendChild(btn);
    });
  }

  function highlightDocsCodeBlocks() {
    var blocks = document.querySelectorAll("pre code[class*='language-zekken']");
    blocks.forEach(highlightCodeElement);
    injectRunButtons();
  }

  window.highlightDocsCodeBlocks = highlightDocsCodeBlocks;
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", highlightDocsCodeBlocks);
  else highlightDocsCodeBlocks();
})();
