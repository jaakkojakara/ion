// A minimal Node.js HTTP server to serve static files
// Sets Cross-Origin-Embedder-Policy and Cross-Origin-Opener-Policy headers for wasm shared memory
const http = require('http');
const fs = require('fs');
const path = require('path');
const { promisify } = require('util');
const readFile = promisify(fs.readFile);
const readdir = promisify(fs.readdir);

const PORT = process.env.PORT || 3000;

const mimeTypes = {
  '.html': 'text/html',
  '.js': 'application/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.wav': 'audio/wav',
  '.mp4': 'video/mp4',
  '.woff': 'application/font-woff',
  '.ttf': 'application/font-ttf',
  '.eot': 'application/vnd.ms-fontobject',
  '.otf': 'application/font-otf',
  '.wasm': 'application/wasm'
};

// PNG header parsing
async function getPngDimensions(filePath) {
  const buffer = await readFile(filePath);
  // PNG signature: 8 bytes
  // IHDR chunk: 4 bytes length + 4 bytes type + 4 bytes width + 4 bytes height + ...
  if (buffer.length < 24) return null;
  
  // Check PNG signature
  if (buffer[0] !== 0x89 || buffer[1] !== 0x50 || buffer[2] !== 0x4E || buffer[3] !== 0x47 ||
      buffer[4] !== 0x0D || buffer[5] !== 0x0A || buffer[6] !== 0x1A || buffer[7] !== 0x0A) {
    return null;
  }
  
  // Read width and height from IHDR chunk (big-endian)
  const width = buffer.readUInt32BE(16);
  const height = buffer.readUInt32BE(20);
  return { width, height };
}

async function getTextureList() {
  const texturesDir = path.join(process.cwd(), 'assets', 'textures');
  const files = await readdir(texturesDir, { withFileTypes: true });
  
  const results = [];
  for (const file of files) {
    if (!file.isFile() || !file.name.toLowerCase().endsWith('.png')) continue;
    
    const filePath = path.join(texturesDir, file.name);
    const dimensions = await getPngDimensions(filePath);
    if (!dimensions) continue;
    
    const relativePath = path.join('assets', 'textures', file.name);
    results.push(`${relativePath}\t${dimensions.width}x${dimensions.height}`);
  }
  
  return results.join('\n');
}

const server = http.createServer(async (req, res) => {
  console.log(`${req.method} ${req.url}`);

  // Headers for wasm shared memory
  res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
  res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');

  // Handle API endpoints
  if (req.url === '/api/textures') {
    try {
      const textureList = await getTextureList();
      res.writeHead(200, { 'Content-Type': 'text/plain' });
      res.end(textureList);
      return;
    } catch (err) {
      console.error('Error processing textures:', err);
      res.writeHead(500, { 'Content-Type': 'text/plain' });
      res.end('500 Internal Server Error');
      return;
    }
  }

  // Handle static files
  let filePath = path.join(process.cwd(), req.url === '/' ? 'index.html' : req.url);
  const ext = path.extname(filePath) || '.html';

  if (!path.extname(filePath)) {
    filePath = path.join(filePath, 'index.html');
  }

  const contentType = mimeTypes[ext.toLowerCase()] || 'application/octet-stream';

  // Read and serve the file
  fs.readFile(filePath, (err, content) => {
    if (err) {
      if (err.code === 'ENOENT') {
        res.writeHead(404, { 'Content-Type': 'text/plain' });
        res.end('404 Not Found');
      } else {
        res.writeHead(500, { 'Content-Type': 'text/plain' });
        res.end('500 Internal Server Error');
      }
    } else {
      res.writeHead(200, { 'Content-Type': contentType });
      res.end(content);
    }
  });
});

server.listen(PORT, () => {
  console.log(`Server running at http://localhost:${PORT}/`);
});
