import { createHash } from 'node:crypto';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, '..');
const skipRuntimeFiles = new Set(['config.json', 'msd.json', 'oris-mania-utils.exe', '.oris-mania-utils-version']);
const overlays = {
    msdconverter: ['index.html', 'main.js', 'styles.css'],
    ManiaKeystrokes: ['index.html', 'keystrokes.js', 'keystrokes.css', 'keystrokes-geometry.js'],
    HitCounter: ['index.html', 'main.js', 'styles.css', 'hitcounter-layout.js'],
};

function sha256(buffer) {
    return createHash('sha256').update(buffer).digest('hex');
}

async function hashFile(filePath) {
    return sha256(await fs.readFile(filePath));
}

async function readJson(filePath) {
    return JSON.parse(await fs.readFile(filePath, 'utf8'));
}

async function exists(filePath) {
    try {
        await fs.access(filePath);
        return true;
    } catch {
        return false;
    }
}

async function listFilesRecursive(rootDir) {
    const result = [];
    const entries = await fs.readdir(rootDir, { withFileTypes: true });
    for (const entry of entries) {
        const fullPath = path.join(rootDir, entry.name);
        if (entry.isDirectory()) {
            result.push(...await listFilesRecursive(fullPath));
        } else {
            result.push(fullPath);
        }
    }
    return result;
}

async function checkOverlaySources() {
    const failures = [];
    for (const [overlayName, files] of Object.entries(overlays)) {
        for (const fileName of files) {
            const fullPath = path.join(repoRoot, 'overlays', overlayName, fileName);
            if (!await exists(fullPath)) {
                failures.push(`Missing overlay source file: overlays/${overlayName}/${fileName}`);
            }
        }
    }
    return failures;
}

async function checkBuildConfig() {
    const configPath = path.join(repoRoot, 'src-tauri', 'tauri.conf.json');
    const config = await readJson(configPath);
    const failures = [];

    if (String(config.identifier || '').endsWith('.app')) {
        failures.push(`Bundle identifier still ends with .app: ${config.identifier}`);
    }

    const targets = config.bundle?.targets;
    if (targets !== 'msi' && !(Array.isArray(targets) && targets.length === 1 && targets[0] === 'msi')) {
        failures.push(`Expected bundle.targets to resolve to MSI-only stable path, found: ${JSON.stringify(targets)}`);
    }

    return failures;
}

async function checkBuildArtifacts() {
    const failures = [];
    const exePath = path.join(repoRoot, 'src-tauri', 'target', 'release', 'oris-mania-utils.exe');
    if (!await exists(exePath)) {
        failures.push('Missing release executable at src-tauri/target/release/oris-mania-utils.exe');
    }

    const msiDir = path.join(repoRoot, 'src-tauri', 'target', 'release', 'bundle', 'msi');
    if (!await exists(msiDir)) {
        failures.push('Missing MSI bundle directory at src-tauri/target/release/bundle/msi');
        return failures;
    }

    const msiFiles = (await fs.readdir(msiDir)).filter((name) => name.endsWith('.msi'));
    if (msiFiles.length === 0) {
        failures.push('No MSI artifact found in src-tauri/target/release/bundle/msi');
    }

    return failures;
}

async function checkInstalledOverlaySync() {
    const failures = [];
    const appConfigPath = path.join(process.env.LOCALAPPDATA || path.join(os.homedir(), 'AppData', 'Local'), 'oris-mania-utils', 'config.json');
    if (!await exists(appConfigPath)) {
        return { skipped: 'No local app config found; installed overlay sync check skipped.', failures };
    }

    const appConfig = await readJson(appConfigPath);
    const tosuRoot = appConfig.tosu_root_path;
    if (!tosuRoot) {
        return { skipped: 'No tosu_root_path in local app config; installed overlay sync check skipped.', failures };
    }

    const staticRoot = path.join(tosuRoot, 'static');
    if (!await exists(staticRoot)) {
        failures.push(`Configured TOSU static folder does not exist: ${staticRoot}`);
        return { failures };
    }

    for (const overlayName of Object.keys(overlays)) {
        const sourceRoot = path.join(repoRoot, 'overlays', overlayName);
        const targetRoot = path.join(staticRoot, overlayName);
        if (!await exists(targetRoot)) {
            failures.push(`Installed overlay folder missing: ${targetRoot}`);
            continue;
        }

        const sourceFiles = await listFilesRecursive(sourceRoot);
        for (const sourceFile of sourceFiles) {
            const relativePath = path.relative(sourceRoot, sourceFile);
            if (skipRuntimeFiles.has(path.basename(relativePath))) {
                continue;
            }
            const targetFile = path.join(targetRoot, relativePath);
            if (!await exists(targetFile)) {
                failures.push(`Installed overlay file missing: ${path.join('static', overlayName, relativePath)}`);
                continue;
            }

            const [sourceHash, targetHash] = await Promise.all([hashFile(sourceFile), hashFile(targetFile)]);
            if (sourceHash !== targetHash) {
                failures.push(`Overlay hash mismatch for ${overlayName}/${relativePath}`);
            }
        }
    }

    return { failures };
}

async function main() {
    const failures = [
        ...await checkOverlaySources(),
        ...await checkBuildConfig(),
        ...await checkBuildArtifacts(),
    ];

    const installCheck = await checkInstalledOverlaySync();
    failures.push(...installCheck.failures);

    const result = {
        ok: failures.length === 0,
        skipped: installCheck.skipped || null,
        failures,
    };

    console.log(JSON.stringify(result, null, 2));
    if (!result.ok) {
        process.exitCode = 1;
    }
}

await main();
