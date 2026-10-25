import { defineStore } from 'pinia'
import { ref } from 'vue'
import axios from 'axios'
import { api } from '@/services/api'

export const useStoreBrandingStore = defineStore('storeBranding', () => {
  const storeName = ref<string>(localStorage.getItem('notane_store_branding_name') || 'NOTANE RETAIL STORE')
  const storeTagline = ref<string>(localStorage.getItem('notane_store_branding_tagline') || 'Pusat Belanja Retail Modern & Terpercaya')
  const storeLogoUrl = ref<string | null>(localStorage.getItem('notane_store_branding_logo') || null)
  const isLoading = ref<boolean>(false)
  const isFetched = ref<boolean>(false)

  function getBaseUrls(): string[] {
    const urls: string[] = []
    const targetHost = (localStorage.getItem('notane_target_host') || '127.0.0.1').trim()
    const targetPort = localStorage.getItem('notane_target_port') || '18080'
    const controlPort = String(Number(targetPort) + 1)

    urls.push(`http://${targetHost}:${targetPort}`)
    urls.push(`http://${targetHost}:${controlPort}`)
    if (!urls.includes('http://127.0.0.1:18080')) urls.push('http://127.0.0.1:18080')
    if (!urls.includes('http://127.0.0.1:18081')) urls.push('http://127.0.0.1:18081')

    if (typeof window !== 'undefined' && window.location?.origin) {
      const orig = window.location.origin
      if (!orig.startsWith('tauri://') && !orig.includes('tauri.localhost') && !orig.includes(':517')) {
        if (!urls.includes(orig)) urls.push(orig)
      }
    }
    return urls
  }

  // Ambil data profil & logo toko dari server host
  async function fetchBranding() {
    isLoading.value = true
    try {
      // 1. Coba endpoint system branding terlebih dahulu (Host Server Control)
      for (const base of getBaseUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/branding`, { timeout: 2500 })
          if (res.data?.success && res.data?.data) {
            const d = res.data.data
            if (d.store_name) {
              storeName.value = d.store_name
              localStorage.setItem('notane_store_branding_name', d.store_name)
            }
            if (d.store_tagline) {
              storeTagline.value = d.store_tagline
              localStorage.setItem('notane_store_branding_tagline', d.store_tagline)
            }
            if (d.store_logo_url) {
              storeLogoUrl.value = d.store_logo_url
              localStorage.setItem('notane_store_branding_logo', d.store_logo_url)
            } else {
              storeLogoUrl.value = null
              localStorage.removeItem('notane_store_branding_logo')
            }
            isFetched.value = true
            return
          }
        } catch (_) {}
      }

      // 2. Fallback: coba endpoint POS settings/receipt
      const res = await api.get('/settings/receipt')
      if (res.data?.success && res.data?.data) {
        const d = res.data.data
        if (d.store_name) {
          storeName.value = d.store_name
          localStorage.setItem('notane_store_branding_name', d.store_name)
        }
        if (d.store_tagline) {
          storeTagline.value = d.store_tagline
          localStorage.setItem('notane_store_branding_tagline', d.store_tagline)
        }
        if (d.store_logo_url) {
          storeLogoUrl.value = d.store_logo_url
          localStorage.setItem('notane_store_branding_logo', d.store_logo_url)
        } else {
          storeLogoUrl.value = null
          localStorage.removeItem('notane_store_branding_logo')
        }
        isFetched.value = true
      }
    } catch (e) {
      console.warn('Gagal mengambil profil branding toko, menggunakan cache lokal:', e)
    } finally {
      isLoading.value = false
    }
  }

  // Upload file gambar logo menjadi URL/Base64
  async function uploadLogoFile(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = async () => {
        const base64Data = reader.result as string
        // 1. Coba upload ke system branding endpoint
        for (const base of getBaseUrls()) {
          try {
            const res = await axios.post(
              `${base}/api/v1/system/branding/upload-logo`,
              {
                filename: file.name,
                data_base64: base64Data,
              },
              { timeout: 6000 }
            )
            if (res.data?.success && (res.data.image_url || res.data.logo_url)) {
              resolve(res.data.image_url || res.data.logo_url)
              return
            }
          } catch (_) {}
        }

        // 2. Coba endpoint master photo jika login token ada
        try {
          const res = await api.post('/products/upload-photo', {
            filename: file.name,
            data_base64: base64Data,
          })
          if (res.data?.success && res.data?.image_url) {
            resolve(res.data.image_url)
            return
          }
        } catch (_) {}

        // 3. Fallback: gunakan base64 langsung
        resolve(base64Data)
      }
      reader.onerror = (err) => reject(err)
      reader.readAsDataURL(file)
    })
  }

  // Simpan perubahan branding ke server host
  async function updateBranding(payload: {
    storeName: string
    storeTagline?: string
    storeLogoUrl?: string | null
  }) {
    isLoading.value = true
    let lastErrorMsg = ''
    try {
      // 1. Coba simpan langsung via system branding endpoint
      for (const base of getBaseUrls()) {
        try {
          const res = await axios.post(
            `${base}/api/v1/system/branding`,
            {
              store_name: payload.storeName,
              store_tagline: payload.storeTagline || '',
              store_logo_url: payload.storeLogoUrl || null,
            },
            { timeout: 6000 }
          )
          if (res.data?.success) {
            storeName.value = payload.storeName
            localStorage.setItem('notane_store_branding_name', payload.storeName)

            if (payload.storeTagline) {
              storeTagline.value = payload.storeTagline
              localStorage.setItem('notane_store_branding_tagline', payload.storeTagline)
            }

            storeLogoUrl.value = payload.storeLogoUrl || null
            if (payload.storeLogoUrl) {
              localStorage.setItem('notane_store_branding_logo', payload.storeLogoUrl)
            } else {
              localStorage.removeItem('notane_store_branding_logo')
            }

            return res.data.data
          }
        } catch (err: any) {
          if (err.response?.data?.message) {
            lastErrorMsg = err.response.data.message
          }
        }
      }

      // 2. Fallback: Coba simpan via POS settings/receipt jika dipanggil dari antarmuka POS (dan sudah login)
      if (localStorage.getItem('notane_token')) {
        try {
          let currentSettings: any = {}
          try {
            const getRes = await api.get('/settings/receipt')
            if (getRes.data?.data) currentSettings = getRes.data.data
          } catch (_) {}

          const updatePayload = {
            ...currentSettings,
            store_name: payload.storeName,
            store_tagline: payload.storeTagline || currentSettings.store_tagline || '',
            store_logo_url: payload.storeLogoUrl !== undefined ? payload.storeLogoUrl : currentSettings.store_logo_url,
          }

          const res = await api.post('/settings/receipt', updatePayload)
          if (res.data?.success) {
            storeName.value = payload.storeName
            localStorage.setItem('notane_store_branding_name', payload.storeName)

            if (payload.storeTagline) {
              storeTagline.value = payload.storeTagline
              localStorage.setItem('notane_store_branding_tagline', payload.storeTagline)
            }

            storeLogoUrl.value = payload.storeLogoUrl || null
            if (payload.storeLogoUrl) {
              localStorage.setItem('notane_store_branding_logo', payload.storeLogoUrl)
            } else {
              localStorage.removeItem('notane_store_branding_logo')
            }

            return res.data.data
          }
        } catch (err: any) {
          if (err.response?.data?.message) {
            lastErrorMsg = err.response.data.message
          }
        }
      }

      throw new Error(lastErrorMsg || 'Gagal menyimpan branding toko ke server host')
    } finally {
      isLoading.value = false
    }
  }

  return {
    storeName,
    storeTagline,
    storeLogoUrl,
    isLoading,
    isFetched,
    fetchBranding,
    uploadLogoFile,
    updateBranding,
  }
})
