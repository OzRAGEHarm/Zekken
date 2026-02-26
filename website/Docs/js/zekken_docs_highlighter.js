(function () {
  function escapeHtml(text) {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
  }

  function isIdentStart(ch) {
    return /[A-Za-z_]/.test(ch);
  }

  function isIdent(ch) {
    return /[A-Za-z0-9_]/.test(ch);
  }

  function isDigit(ch) {
    return /[0-9]/.test(ch);
  }

  function tokenizeLine(line) {
    var out = "";
    var i = 0;

    while (i < line.length) {
      var ch = line[i];
      var next = i + 1 < line.length ? line[i + 1] : "";

      if (ch === "/" && next === "/") {
        out += '<span class="tok-comment">' + escapeHtml(line.slice(i)) + '</span>';
        break;
      }

      if (ch === '"' || ch === "'") {
        var quote = ch;
        var j = i + 1;
        var escaped = false;
        while (j < line.length) {
          var cj = line[j];
          if (escaped) {
            escaped = false;
          } else if (cj === "\\") {
            escaped = true;
          } else if (cj === quote) {
            j++;
            break;
          }
          j++;
        }
        out += '<span class="tok-string">' + escapeHtml(line.slice(i, j)) + '</span>';
        i = j;
        continue;
      }

      if (isDigit(ch)) {
        var k = i;
        while (k < line.length && isDigit(line[k])) k++;
        if (k < line.length && line[k] === ".") {
          var m = k + 1;
          var hasFrac = false;
          while (m < line.length && isDigit(line[m])) {
            hasFrac = true;
            m++;
          }
          if (hasFrac) k = m;
        }
        out += '<span class="tok-number">' + escapeHtml(line.slice(i, k)) + '</span>';
        i = k;
        continue;
      }

      if (ch === "@") {
        out += "@";
        i++;
        var bStart = i;
        while (i < line.length && isIdent(line[i])) i++;
        if (i > bStart) {
          out += '<span class="tok-builtin">' + escapeHtml(line.slice(bStart, i)) + '</span>';
        }
        continue;
      }

      if (isIdentStart(ch)) {
        var idEnd = i + 1;
        while (idEnd < line.length && isIdent(line[idEnd])) idEnd++;
        var ident = line.slice(i, idEnd);
        var rest = line.slice(idEnd);

        var cls = "tok-variable";
        if (/^(if|else|for|while|try|catch|return)$/.test(ident)) {
          cls = "tok-keyword-control";
        } else if (/^(use|include|export|from|in|let|const)$/.test(ident)) {
          cls = "tok-keyword";
        } else if (/^(int|float|bool|string|arr|obj|fn)$/.test(ident)) {
          cls = "tok-type";
        } else if (/^(true|false)$/.test(ident)) {
          cls = "tok-boolean";
        } else if (/^\s*=>/.test(rest)) {
          cls = "tok-function";
        }

        if (ident === "func") {
          cls = "tok-keyword";
          out += '<span class="' + cls + '">' + escapeHtml(ident) + '</span>';
          i = idEnd;

          while (i < line.length && /\s/.test(line[i])) {
            out += escapeHtml(line[i]);
            i++;
          }

          var fnStart = i;
          while (i < line.length && isIdent(line[i])) i++;
          if (i > fnStart) {
            out += '<span class="tok-function">' + escapeHtml(line.slice(fnStart, i)) + '</span>';
          }
          continue;
        }

        out += '<span class="' + cls + '">' + escapeHtml(ident) + '</span>';
        i = idEnd;
        continue;
      }

      if (/[+\-*/%=<>!|]+/.test(ch)) {
        var opEnd = i + 1;
        while (opEnd < line.length && /[+\-*/%=<>!|]/.test(line[opEnd])) opEnd++;
        out += '<span class="tok-operator">' + escapeHtml(line.slice(i, opEnd)) + '</span>';
        i = opEnd;
        continue;
      }

      out += escapeHtml(ch);
      i++;
    }

    return out;
  }

  function highlightCodeElement(codeEl) {
    var source = codeEl.textContent || "";
    var lines = source.split("\n");
    var html = [];
    for (var i = 0; i < lines.length; i++) {
      html.push(tokenizeLine(lines[i]));
    }
    codeEl.classList.add("zk-code");
    codeEl.innerHTML = html.join("\n");
  }

  function highlightDocsCodeBlocks() {
    var blocks = document.querySelectorAll("pre code.language-zekken");
    blocks.forEach(highlightCodeElement);
  }

  window.highlightDocsCodeBlocks = highlightDocsCodeBlocks;

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", highlightDocsCodeBlocks);
  } else {
    highlightDocsCodeBlocks();
  }
})();
