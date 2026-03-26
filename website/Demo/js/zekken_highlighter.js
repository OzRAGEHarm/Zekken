CodeMirror.defineMode("zekken", function() {
  return {
    token: function(stream, state) {
      if (stream.sol()) {
        state.expectFuncName = false;
      }

      // Comments
      if (stream.match("//")) {
        stream.skipToEnd();
        return "comment";
      }
      
      if (stream.match("/*")) {
        state.inComment = true;
        return "comment";
      }
      
      if (state.inComment) {
        if (stream.match("*/")) {
          state.inComment = false;
        } else {
          stream.next();
        }
        return "comment";
      }

      // Strings
      if (stream.match(/"(?:[^"\\]|\\.)*"/)) return "string";
      if (stream.match(/'(?:[^'\\]|\\.)*'/)) return "string";
      
      // Keep '@' unstyled, but mark next identifier as builtin/function name.
      if (stream.peek() === "@") {
        stream.next();
        state.afterAt = true;
        return null;
      }

      // Dot accessor: mark next identifier as property/member name.
      if (stream.peek() === ".") {
        stream.next();
        state.afterDot = true;
        return "operator";
      }

      // Identifier handling with context.
      if (stream.match(/[a-zA-Z_][a-zA-Z0-9_]*/)) {
        const ident = stream.current();

        // Declaration name after 'func'
        if (state.expectFuncName) {
          state.expectFuncName = false;
          return "function";
        }

        // Built-in name right after '@'
        if (state.afterAt) {
          state.afterAt = false;
          return "builtin";
        }

        // Member/property right after dot.
        if (state.afterDot) {
          state.afterDot = false;
          if (stream.match(/\s*=>/, false)) return "function";
          if (/^(PI|E|I)$/.test(ident)) return "constant";
          return "property";
        }

        // Function/method call name before =>
        if (stream.match(/\s*=>/, false)) {
          return "function";
        }

        // Keywords
        if (/^(if|else|for|while|try|catch|return)$/.test(ident)) return "keyword-control";
        if (/^(use|include|export|from|in|let|const)$/.test(ident)) return "keyword";
        if (ident === "func") {
          state.expectFuncName = true;
          return "keyword";
        }

        // Types
        if (/^(int|float|bool|string|arr|obj|fn)$/.test(ident)) return "type";

        // Booleans
        if (/^(true|false)$/.test(ident)) return "boolean";

        // Variables
        return "variable";
      }

      // Function names (identifier before '=>', but not after '@')
      if (stream.match(/\b[a-zA-Z_][a-zA-Z0-9_]*\b(?=\s*=>)/)) return "function";
      
      // Numbers
      if (stream.match(/\b\d+\.\d+\b/)) return "number";
      if (stream.match(/\b\d+\b/)) return "number";
      
      // Operators
      if (stream.match(/[+\-*/%=<>!|]+/)) return "operator";
      
      stream.next();
      return null;
    },
    startState: function() {
      return {
        inComment: false,
        afterAt: false,
        afterDot: false,
        expectFuncName: false
      };
    }
  };
});

CodeMirror.defineMIME("text/x-zekken", "zekken");
