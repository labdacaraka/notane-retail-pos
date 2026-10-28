#!/usr/bin/env node

/**
 * scripts/sync-version.js
 * 
 * Synchronizes the application version across all package manifests,
 * Tauri configurations, and Cargo.toml definitions in the repository.
 * 
 * Target files:
 * 1. package.json (root, notane-server, notane-client, notane-manager)
 * 2. tauri.conf.json (notane-server, notane-client, notane-manager)
 * 3. Cargo.toml (notane-server, notane-client, notane-manager)
 */

import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const ROOT_DIR = path.resolve(__dirname, '..')

let targetVersion = process.argv[2]

if (!targetVersion) {
  console.error('❌ Error: No version specified.')
  console.error('Usage: node scripts/sync-version.js <version> (e.g., node scripts/sync-version.js 1.4.1)')
  process.exit(1)
}

// Strip leading 'v' if present (e.g., "v1.4.1" -> "1.4.1")
targetVersion = targetVersion.replace(/^v/i, '').trim()

// SemVer validation regex
const semverRegex = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/
if (!semverRegex.test(targetVersion)) {
  console.warn(`⚠️ Warning: "${targetVersion}" does not strictly match standard SemVer format (MAJOR.MINOR.PATCH). Proceeding anyway...`)
}

console.log(`\n======================================================`)
console.log(` 🔄 Synchronizing Project Version to: ${targetVersion}`)
console.log(`======================================================\n`)

const packageJsonPaths = [
  path.join(ROOT_DIR, 'package.json'),
  path.join(ROOT_DIR, 'notane-server', 'package.json'),
  path.join(ROOT_DIR, 'notane-client', 'package.json'),
  path.join(ROOT_DIR, 'notane-manager', 'package.json'),
]

const tauriConfPaths = [
  path.join(ROOT_DIR, 'notane-server', 'src-tauri', 'tauri.conf.json'),
  path.join(ROOT_DIR, 'notane-client', 'src-tauri', 'tauri.conf.json'),
  path.join(ROOT_DIR, 'notane-manager', 'src-tauri', 'tauri.conf.json'),
]

const cargoTomlPaths = [
  path.join(ROOT_DIR, 'notane-server', 'src-tauri', 'Cargo.toml'),
  path.join(ROOT_DIR, 'notane-client', 'src-tauri', 'Cargo.toml'),
  path.join(ROOT_DIR, 'notane-manager', 'src-tauri', 'Cargo.toml'),
]

let updatedCount = 0

// 1. Update package.json files
for (const p of packageJsonPaths) {
  if (fs.existsSync(p)) {
    try {
      const content = fs.readFileSync(p, 'utf8')
      const json = JSON.parse(content)
      const oldVersion = json.version
      json.version = targetVersion
      fs.writeFileSync(p, JSON.stringify(json, null, 2) + '\n', 'utf8')
      console.log(`  ✔ [package.json] ${path.relative(ROOT_DIR, p)}: ${oldVersion} -> ${targetVersion}`)
      updatedCount++
    } catch (err) {
      console.error(`  ❌ Failed updating ${path.relative(ROOT_DIR, p)}: ${err.message}`)
    }
  }
}

// 2. Update tauri.conf.json files
for (const p of tauriConfPaths) {
  if (fs.existsSync(p)) {
    try {
      const content = fs.readFileSync(p, 'utf8')
      const json = JSON.parse(content)
      const oldVersion = json.version
      json.version = targetVersion
      fs.writeFileSync(p, JSON.stringify(json, null, 2) + '\n', 'utf8')
      console.log(`  ✔ [tauri.conf.json] ${path.relative(ROOT_DIR, p)}: ${oldVersion} -> ${targetVersion}`)
      updatedCount++
    } catch (err) {
      console.error(`  ❌ Failed updating ${path.relative(ROOT_DIR, p)}: ${err.message}`)
    }
  }
}

// 3. Update Cargo.toml files
for (const p of cargoTomlPaths) {
  if (fs.existsSync(p)) {
    try {
      const content = fs.readFileSync(p, 'utf8')
      // Only replace version inside the [package] table
      const packageRegex = /(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m
      if (packageRegex.test(content)) {
        const updated = content.replace(packageRegex, `$1${targetVersion}$2`)
        fs.writeFileSync(p, updated, 'utf8')
        console.log(`  ✔ [Cargo.toml] ${path.relative(ROOT_DIR, p)} -> ${targetVersion}`)
        updatedCount++
      } else {
        console.warn(`  ⚠️ Could not find [package] version field in ${path.relative(ROOT_DIR, p)}`)
      }
    } catch (err) {
      console.error(`  ❌ Failed updating ${path.relative(ROOT_DIR, p)}: ${err.message}`)
    }
  }
}

console.log(`\n🎉 Successfully synchronized ${updatedCount} configuration files to version ${targetVersion}.\n`)
