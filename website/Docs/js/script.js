// Docs-only behavior: clicking the hover '#' next to a heading copies a deep-link.
(function () {
  function copyText(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      return navigator.clipboard.writeText(text).then(
        function () { return true; },
        function () { return false; }
      );
    }

    // Fallback for older browsers / non-secure contexts.
    try {
      var ta = document.createElement("textarea");
      ta.value = text;
      ta.setAttribute("readonly", "true");
      ta.style.position = "fixed";
      ta.style.left = "-9999px";
      ta.style.top = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      return Promise.resolve(true);
    } catch (_) {
      return Promise.resolve(false);
    }
  }

  function baseUrl() {
    return window.location.href.split("#")[0];
  }

  document.addEventListener("DOMContentLoaded", function () {
    var links = document.querySelectorAll("a.doc-anchor-hash[href^=\"#\"]");
    for (var i = 0; i < links.length; i++) {
      (function (a) {
        a.addEventListener("click", function (ev) {
          ev.preventDefault();

          var href = a.getAttribute("href") || "";
          var url = baseUrl() + href;
          var id = href.slice(1);

          copyText(url).then(function () {
            // Navigate too, so the section is targeted/highlighted.
            if (id) window.location.hash = id;

            a.setAttribute("data-copied", "1");
            window.setTimeout(function () {
              a.removeAttribute("data-copied");
            }, 1100);

            // Don't leave the '#' link in a focused/selected-looking state.
            try {
              if (window.getSelection) {
                var sel = window.getSelection();
                if (sel && sel.removeAllRanges) sel.removeAllRanges();
              }
            } catch (_) {}
            try { a.blur(); } catch (_) {}
          });
        });
      })(links[i]);
    }
  });
})();
