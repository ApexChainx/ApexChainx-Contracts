const fs = require('fs');
const path = require('path');

/**
 * Normalizes snapshot files to ensure cross-platform reproducibility (Windows / Linux / macOS)
 */
function normalizeContent(content) {
  return content
    // Convert CRLF to LF
    .replace(/\r\n/g, '\n')
    // Normalize Windows backslashes in paths
    .replace(/\\\\/g, '/')
    // Strip ANSI color codes
    .replace(/\u001b\[[0-9;]*m/g, '');
}

function processDirectory(dirPath) {
  if (!fs.existsSync(dirPath)) return;

  const entries = fs.readdirSync(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    const fullPath = path.join(dirPath, entry.name);
    if (entry.isDirectory()) {
      processDirectory(fullPath);
    } else if (entry.name.endsWith('.snap') || entry.name.endsWith('.snapshot')) {
      const raw = fs.readFileSync(fullPath, 'utf8');
      const normalized = normalizeContent(raw);
      if (raw !== normalized) {
        fs.writeFileSync(fullPath, normalized, 'utf8');
        console.log(`Normalized snapshot: ${fullPath}`);
      }
    }
  }
}

const testsDir = path.resolve(__dirname, '../tests');
const calculatorTestsDir = path.resolve(__dirname, '../apexchainx_calculator/src');

console.log('Starting snapshot normalization...');
processDirectory(testsDir);
processDirectory(calculatorTestsDir);
console.log('Snapshot normalization complete.');
