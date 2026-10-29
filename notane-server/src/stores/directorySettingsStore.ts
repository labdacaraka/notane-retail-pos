import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface SystemStoragePaths {
  downloads: string
  documents: string
  desktop: string
  default_export_dir: string
  default_backup_dir: string
}

export const useDirectorySettingsStore = defineStore('directorySettings', () => {
  const systemPaths = ref<SystemStoragePaths | null>(null)
  const exportDirectory = ref<string>(localStorage.getItem('notane_export_directory') || '')
  const backupDirectory = ref<string>(localStorage.getItem('notane_backup_directory') || '')
  const isLoaded = ref<boolean>(false)

  const isTauri = computed(() => {
    if (typeof window === 'undefined') return false
    return Boolean(
      (window as any).__TAURI_INTERNALS__ ||
      (window as any).__TAURI__ ||
      (window as any).__TAURI_INVOKE__
    )
  })

  async function init() {
    if (isLoaded.value) return

    if (isTauri.value) {
      try {
        const { invoke } = await import('@tauri-apps/api/core')
        const paths = await invoke<SystemStoragePaths>('get_system_storage_paths')
        systemPaths.value = paths

        if (!exportDirectory.value && paths.default_export_dir) {
          exportDirectory.value = paths.default_export_dir
          localStorage.setItem('notane_export_directory', paths.default_export_dir)
        }
        if (!backupDirectory.value && paths.default_backup_dir) {
          backupDirectory.value = paths.default_backup_dir
          localStorage.setItem('notane_backup_directory', paths.default_backup_dir)
        }
      } catch (err) {
        console.warn('[DirectoryStore] Gagal mengambil path sistem bawaan:', err)
      }
    }

    isLoaded.value = true
  }

  function setExportDirectory(path: string) {
    const trimmed = path.trim()
    exportDirectory.value = trimmed
    if (trimmed) {
      localStorage.setItem('notane_export_directory', trimmed)
    } else {
      localStorage.removeItem('notane_export_directory')
    }
  }

  function setBackupDirectory(path: string) {
    const trimmed = path.trim()
    backupDirectory.value = trimmed
    if (trimmed) {
      localStorage.setItem('notane_backup_directory', trimmed)
    } else {
      localStorage.removeItem('notane_backup_directory')
    }
  }

  function resetExportDirectory() {
    if (systemPaths.value?.default_export_dir) {
      setExportDirectory(systemPaths.value.default_export_dir)
    } else {
      setExportDirectory('')
    }
  }

  function resetBackupDirectory() {
    if (systemPaths.value?.default_backup_dir) {
      setBackupDirectory(systemPaths.value.default_backup_dir)
    } else {
      setBackupDirectory('')
    }
  }

  async function chooseExportDirectory(): Promise<string | null> {
    if (!isTauri.value) return null
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const chosen = await invoke<string | null>('select_folder', {
        defaultPath: exportDirectory.value || systemPaths.value?.default_export_dir || null,
      })
      if (chosen) {
        setExportDirectory(chosen)
        return chosen
      }
    } catch (err) {
      console.error('[DirectoryStore] Gagal memilih folder ekspor:', err)
    }
    return null
  }

  async function chooseBackupDirectory(): Promise<string | null> {
    if (!isTauri.value) return null
    try {
      const { invoke } = await import('@tauri-apps/api/core')
      const chosen = await invoke<string | null>('select_folder', {
        defaultPath: backupDirectory.value || systemPaths.value?.default_backup_dir || null,
      })
      if (chosen) {
        setBackupDirectory(chosen)
        return chosen
      }
    } catch (err) {
      console.error('[DirectoryStore] Gagal memilih folder backup:', err)
    }
    return null
  }

  async function openExportFolder(): Promise<boolean> {
    const targetDir = exportDirectory.value || systemPaths.value?.default_export_dir || systemPaths.value?.downloads
    if (!targetDir || !isTauri.value) return false

    try {
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('open_folder', { folderPath: targetDir })
      return true
    } catch (err) {
      console.error('[DirectoryStore] Gagal membuka folder ekspor:', err)
      return false
    }
  }

  async function openBackupFolder(): Promise<boolean> {
    const targetDir = backupDirectory.value || systemPaths.value?.default_backup_dir || systemPaths.value?.downloads
    if (!targetDir || !isTauri.value) return false

    try {
      const { invoke } = await import('@tauri-apps/api/core')
      await invoke('open_folder', { folderPath: targetDir })
      return true
    } catch (err) {
      console.error('[DirectoryStore] Gagal membuka folder backup:', err)
      return false
    }
  }

  async function saveBackupFile(
    filename: string,
    fileBlobOrUrl: Blob | string
  ): Promise<{ success: boolean; filePath: string; folderPath: string; isNativeSave: boolean }> {
    await init()

    const targetDir = backupDirectory.value || systemPaths.value?.default_backup_dir

    if (isTauri.value && targetDir) {
      try {
        let base64Data = ''
        if (typeof fileBlobOrUrl === 'string') {
          const response = await fetch(fileBlobOrUrl)
          const blob = await response.blob()
          base64Data = await blobToBase64(blob)
        } else {
          base64Data = await blobToBase64(fileBlobOrUrl)
        }

        const { invoke } = await import('@tauri-apps/api/core')
        const fullSavedPath = await invoke<string>('save_file_to_directory', {
          directory: targetDir,
          filename,
          base64Data,
        })

        return {
          success: true,
          filePath: fullSavedPath,
          folderPath: targetDir,
          isNativeSave: true,
        }
      } catch (err) {
        console.warn('[DirectoryStore] Simpan native backup gagal, fallback ke browser download:', err)
      }
    }

    let downloadUrl = typeof fileBlobOrUrl === 'string' ? fileBlobOrUrl : URL.createObjectURL(fileBlobOrUrl)
    const link = document.createElement('a')
    link.href = downloadUrl
    link.download = filename
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    if (typeof fileBlobOrUrl !== 'string') {
      URL.revokeObjectURL(downloadUrl)
    }

    return {
      success: true,
      filePath: filename,
      folderPath: systemPaths.value?.downloads || 'Folder Unduhan Browser',
      isNativeSave: false,
    }
  }

  function blobToBase64(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onloadend = () => {
        const result = reader.result as string
        const base64 = result.includes(',') ? result.split(',')[1] : result
        resolve(base64)
      }
      reader.onerror = reject
      reader.readAsDataURL(blob)
    })
  }

  return {
    systemPaths,
    exportDirectory,
    backupDirectory,
    isLoaded,
    isTauri,
    init,
    setExportDirectory,
    setBackupDirectory,
    resetExportDirectory,
    resetBackupDirectory,
    chooseExportDirectory,
    chooseBackupDirectory,
    openExportFolder,
    openBackupFolder,
    saveBackupFile,
  }
})

