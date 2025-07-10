const ansiToHtml = (text) => {
  // Map ANSI codes to CSS classes
  const ansiMap = {
    '1;31m': 'ansi-red',
    '1;35m': 'ansi-magenta',
    '1;33m': 'ansi-yellow',
    '1;34m': 'ansi-blue',
    '1;41m': 'ansi-red',
    '1;37m': 'ansi-location',
    '1;90m': 'ansi-gray',
    '1;32m': 'ansi-green',
    '38;2;106;153;85m': 'ansi-comment',
    '38;2;86;156;214m': 'ansi-keyword',
    '38;2;220;220;170m': 'ansi-func',
    '38;2;198;120;221m': 'ansi-control',
    '38;2;156;220;254m': 'ansi-var',
    '38;2;78;201;176m': 'ansi-type',
    '38;2;181;206;168m': 'ansi-number',
    '38;2;206;145;120m': 'ansi-string',
  };

  // Replace ANSI codes with <span class="...">
  return text.replace(/\x1b\[([^m]+)m([^\x1b]*)/g, (_, code, content) => {
    const cls = ansiMap[code] || '';
    return cls ? `<span class="${cls}">${content}</span>` : content;
  }).replace(/\x1b\[0m/g, '</span>');
};

export { ansiToHtml };
