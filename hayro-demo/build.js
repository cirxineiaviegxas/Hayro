const { execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

// Build the Rust code to WASM
console.log('Building Rust code to WASM...');
execSync('wasm-pack build --target web', { stdio: 'inherit' });

// Copy the WASM files to the www directory
console.log('Copying WASM files to www directory...');
const pkgDir = path.join(__dirname, 'pkg');
const wwwDir = path.join(__dirname, 'www');

if (!fs.existsSync(wwwDir)) {
    fs.mkdirSync(wwwDir);
}

fs.copyFileSync(
    path.join(pkgDir, 'hayro_demo_bg.wasm'),
    path.join(wwwDir, 'hayro_demo_bg.wasm')
);

fs.copyFileSync(
    path.join(pkgDir, 'hayro_demo.js'),
    path.join(wwwDir, 'hayro_demo.js')
);

console.log('Build complete!'); 