#!/usr/bin/env node
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const isWindows = process.platform === 'win32';
const args = process.argv.slice(2);

let cmd = '';
let cmdArgs = [];

if (isWindows) {
  cmd = 'powershell.exe';
  const ps1Script = path.join(__dirname, 'brand-manager.ps1');
  cmdArgs = ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', ps1Script, ...args];
} else {
  cmd = 'bash';
  const shScript = path.join(__dirname, 'brand-manager.sh');
  cmdArgs = [shScript, ...args];
}

const envPath = `/opt/podman/bin:/usr/local/bin:/opt/homebrew/bin:${process.env.PATH || ''}`;

const child = spawn(cmd, cmdArgs, {
  stdio: 'inherit',
  shell: isWindows,
  env: {
    ...process.env,
    PATH: envPath,
  },
});

child.on('exit', (code) => {
  process.exit(code ?? 0);
});
