CodeMirror.defineMode("zekken", function() {
  return {
    token: function(stream, state) {
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
      
      // Built-in functions (@println etc)
      if (stream.match(/@([a-zA-Z_][a-zA-Z0-9_]*)/)) {
        // If immediately followed by '=>', treat as builtin
        const pos = stream.pos;
        if (stream.match(/\s*=>/, false)) {
          stream.backUp(stream.pos - pos); // reset to after '@name'
          return "builtin";
        } else {
          stream.backUp(stream.pos - pos); // reset to after '@name'
          return "variable"; // treat as variable if not a call
        }
      }

      // Function names (identifier before '=>', but not after '@')
      if (stream.match(/\b[a-zA-Z_][a-zA-Z0-9_]*\b(?=\s*=>)/)) return "function";

      // Keywords
      if (stream.match(/\b(if|else|for|while|try|catch|return)\b/)) return "keyword-control";
      if (stream.match(/\b(use|include|export|func|let|const|from|in)\b/)) return "keyword";
      
      // Types
      if (stream.match(/\b(int|float|bool|string|arr|obj|fn)\b/)) return "type";
      
      // Numbers
      if (stream.match(/\b\d+\.\d+\b/)) return "number";
      if (stream.match(/\b\d+\b/)) return "number";
      
      // Booleans
      if (stream.match(/\b(true|false)\b/)) return "boolean";
      
      // Operators
      if (stream.match(/[+\-*/%=<>!|]+/)) return "operator";
      
      // Variables
      if (stream.match(/\b[a-zA-Z_][a-zA-Z0-9_]*\b/)) return "variable";
      
      stream.next();
      return null;
    },
    startState: function() {
      return { inComment: false };
    }
  };
});

CodeMirror.defineMIME("text/x-zekken", "zekken");
