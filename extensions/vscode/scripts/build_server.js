// @ts-nocheck
const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');

function run(cmd, args) {
    const r = spawnSync(cmd, args, { stdio: 'inherit' });
    if (r.status !== 0) {
        throw new Error(`${cmd} ${args.join(' ')} failed with exit ${r.status}`);
    }
}

function main() {
    const argv = process.argv.slice(2);
    const tIdx = argv.indexOf('--target');
    const target = (tIdx >= 0 && argv[tIdx + 1]) ? argv[tIdx + 1] : undefined;
    const release = argv.includes('--release');

    // repo root from this script location: ../../../
    const repoRoot = path.resolve(__dirname, '..', '..', '..');
    const exeExt = process.platform === 'win32' ? '.exe' : '';
    const binName = `path-server${exeExt}`;

    console.log('Repo root:', repoRoot);
    if (target) {
        console.log('Building for target:', target, release ? '(release)' : '(debug)');
        run('rustup', ['target', 'add', target]);
    } else {
        console.log(release ? 'Building native release' : 'Building native debug');
    }

    const cargoArgs = ['build'];
    if (release) { cargoArgs.push('--release'); }
    if (target) { cargoArgs.push('--target', target); }
    run('cargo', cargoArgs);

    const profileDir = release ? 'release' : 'debug';
    const src = target
        ? path.join(repoRoot, 'target', target, profileDir, binName)
        : path.join(repoRoot, 'target', profileDir, binName);

    const destDir = path.join(repoRoot, 'extensions', 'vscode', 'bin');
    const dest = path.join(destDir, binName);

    if (!fs.existsSync(src)) {
        throw new Error(`Built binary not found: ${src}`);
    }

    fs.mkdirSync(destDir, { recursive: true });
    fs.copyFileSync(src, dest);
    try {
        if (process.platform !== 'win32') { fs.chmodSync(dest, 0o755); }
    } catch (e) {
        // ignore chmod on platforms that don't support it
    }

    console.log(`Copied ${src} -> ${dest}`);
}

try {
    main();
} catch (err) {
    console.error('build_server failed:', err);
    process.exit(1);
}