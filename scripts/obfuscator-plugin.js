import JavaScriptObfuscator from 'javascript-obfuscator'

/**
 * Enterprise Production Code Hardening Plugin for Vite
 * 
 * Provides Defense-in-Depth anti-reverse-engineering protections targeted specifically
 * at proprietary business logic (src/), stores, API calls, and cryptography checks:
 * - Control flow flattening (spaghetti logic flow confusing AI decompiler analysis)
 * - String literal extraction & Base64 encoding
 * - Number expression transformation
 * - Object key transformation
 * - Console output suppression (prevents sniffing IPC / licensing payloads)
 */
export function codeHardeningPlugin(userOptions = {}) {
  const isEnabled = process.env.ENABLE_OBFUSCATION !== 'false' && process.env.NODE_ENV !== 'development'

  if (!isEnabled) {
    return {
      name: 'notane-code-hardening-noop',
    }
  }

  return {
    name: 'notane-code-hardening',
    apply: 'build',
    enforce: 'post',
    transform(code, id) {
      const cleanId = id.split('?')[0]
      // Hanya proteksi kode sumber aplikasi (src/), lewati vendor third-party di node_modules
      if (
        !cleanId.includes('/src/') ||
        cleanId.includes('node_modules') ||
        cleanId.endsWith('.json') ||
        cleanId.endsWith('.css')
      ) {
        return null
      }

      if (!cleanId.endsWith('.ts') && !cleanId.endsWith('.js') && !cleanId.endsWith('.vue')) {
        return null
      }

      try {
        const obfuscated = JavaScriptObfuscator.obfuscate(code, {
          compact: true,
          controlFlowFlattening: true,
          controlFlowFlatteningThreshold: 0.6,
          deadCodeInjection: false,
          numbersToExpressions: true,
          simplify: true,
          stringArray: true,
          stringArrayEncoding: ['base64'],
          stringArrayThreshold: 0.8,
          splitStrings: true,
          splitStringsChunkLength: 10,
          transformObjectKeys: true,
          unicodeEscapeSequence: false,
          disableConsoleOutput: true,
          identifierNamesGenerator: 'hexadecimal',
          ...userOptions,
        })

        return {
          code: obfuscated.getObfuscatedCode(),
          map: null,
        }
      } catch (err) {
        return null
      }
    },
  }
}
