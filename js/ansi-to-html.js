const ansiToHtml = (text) => {
  const ansiColors = {
    // Base colors
    '1;31m': 'color:#FF5555;font-weight:bold', // Error red
    '1;35m': 'color:#FF79C6;font-weight:bold', // Runtime magenta
    '1;33m': 'color:#F1FA8C;font-weight:bold', // Type yellow
    '1;34m': 'color:#6272A4;font-weight:bold', // Reference blue
    '1;41m': 'color:#FF5555;font-weight:bold', // Internal error
    '1;37m': 'color:#F8F8F2;font-weight:bold', // Location
    '1;90m': 'color:#6272A4;font-weight:bold', // Gray text
    '1;32m': 'color:#50FA7B;font-weight:bold', // Success green
    
    // Syntax colors
    '38;2;106;153;85m': 'color:#6A9955', // Comments
    '38;2;86;156;214m': 'color:#569CD6',  // Keywords
    '38;2;220;220;170m': 'color:#DCDCAA', // Functions
    '38;2;198;120;221m': 'color:#C678DD', // Control
    '38;2;156;220;254m': 'color:#9CDCFE', // Variables
    '38;2;78;201;176m': 'color:#4EC9B0',  // Types
    '38;2;181;206;168m': 'color:#B5CEA8', // Numbers
    '38;2;206;145;120m': 'color:#CE9178', // Strings
  };

  return text.replace(/\x1b\[([^m]+)m([^\x1b]*)/g, (_, code, content) => {
    const style = ansiColors[code] || '';
    return `<span style="${style}">${content}</span>`;
  });
};

export { ansiToHtml };
