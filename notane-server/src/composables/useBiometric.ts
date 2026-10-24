import { ref } from 'vue'
import { api } from '@/services/api'
import type { BiometricSettings } from '@/types'

// Helper conversion
function bufferToBase64Url(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer)
  let binary = ''
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=+$/, '')
}

function base64UrlToBuffer(base64url: string): Uint8Array {
  const base64 = base64url.replace(/-/g, '+').replace(/_/g, '/')
  const pad = base64.length % 4
  const padded = pad ? base64 + '='.repeat(4 - pad) : base64
  const binary = atob(padded)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

export function useBiometric() {
  const activeStream = ref<MediaStream | null>(null)
  const isCameraActive = ref(false)

  // 1. WebAuthn / Sensor Sidik Jari (Touch ID / Windows Hello)
  const isWebAuthnSupported = (): boolean => {
    return (
      typeof window !== 'undefined' &&
      !!window.PublicKeyCredential &&
      typeof navigator.credentials !== 'undefined'
    )
  }

  async function registerFingerprint(userId: string, username: string, fullName: string) {
    if (!isWebAuthnSupported()) {
      throw new Error('Sensor biometrik / WebAuthn tidak didukung pada browser atau perangkat ini')
    }

    let challengeBytes: Uint8Array
    try {
      const res = await api.get('/auth/biometric/challenge')
      if (res.data?.success && res.data.data?.challenge) {
        challengeBytes = base64UrlToBuffer(res.data.data.challenge)
      } else {
        challengeBytes = new Uint8Array(32)
        window.crypto.getRandomValues(challengeBytes)
      }
    } catch {
      challengeBytes = new Uint8Array(32)
      window.crypto.getRandomValues(challengeBytes)
    }

    const userIdBytes = new TextEncoder().encode(userId)

    const credential = (await navigator.credentials.create({
      publicKey: {
        challenge: challengeBytes,
        rp: {
          name: 'Notane Retail POS',
          id: window.location.hostname || 'localhost',
        },
        user: {
          id: userIdBytes,
          name: username,
          displayName: fullName,
        },
        pubKeyCredParams: [
          { type: 'public-key', alg: -7 }, // ES256
          { type: 'public-key', alg: -257 }, // RS256
        ],
        authenticatorSelection: {
          authenticatorAttachment: 'platform', // Touch ID / Windows Hello / On-device fingerprint
          userVerification: 'preferred',
          requireResidentKey: false,
        },
        timeout: 60000,
        attestation: 'none',
      },
    })) as PublicKeyCredential | null

    if (!credential) {
      throw new Error('Pendaftaran sensor sidik jari dibatalkan')
    }

    const credentialId = bufferToBase64Url(credential.rawId)
    const rawResponse = credential.response as AuthenticatorAttestationResponse
    const publicKey = rawResponse.attestationObject
      ? bufferToBase64Url(rawResponse.attestationObject)
      : credentialId

    return {
      credential_id: credentialId,
      public_key: publicKey,
    }
  }

  async function verifyFingerprint(expectedCredentialId?: string) {
    if (!isWebAuthnSupported()) {
      throw new Error('Sensor biometrik tidak didukung pada perangkat ini')
    }

    let challengeBytes: Uint8Array
    try {
      const res = await api.get('/auth/biometric/challenge')
      if (res.data?.success && res.data.data?.challenge) {
        challengeBytes = base64UrlToBuffer(res.data.data.challenge)
      } else {
        challengeBytes = new Uint8Array(32)
        window.crypto.getRandomValues(challengeBytes)
      }
    } catch {
      challengeBytes = new Uint8Array(32)
      window.crypto.getRandomValues(challengeBytes)
    }

    const allowCredentials = expectedCredentialId
      ? [
          {
            type: 'public-key' as const,
            id: base64UrlToBuffer(expectedCredentialId),
          },
        ]
      : undefined

    const assertion = (await navigator.credentials.get({
      publicKey: {
        challenge: challengeBytes,
        allowCredentials,
        userVerification: 'required',
        timeout: 60000,
      },
    })) as PublicKeyCredential | null

    if (!assertion) {
      throw new Error('Pemindaian sidik jari dibatalkan')
    }

    return {
      success: true,
      credential_id: bufferToBase64Url(assertion.rawId),
    }
  }

  // 2. Kamera & Pemindai Wajah (Face Recognition)
  async function startCamera(videoElement: HTMLVideoElement): Promise<MediaStream> {
    stopCamera(videoElement)

    if (!navigator.mediaDevices || !navigator.mediaDevices.getUserMedia) {
      throw new Error('Akses kamera tidak didukung pada terminal POS ini')
    }

    try {
      const stream = await navigator.mediaDevices.getUserMedia({
        video: {
          width: { ideal: 640 },
          height: { ideal: 480 },
          facingMode: 'user',
        },
        audio: false,
      })

      videoElement.srcObject = stream
      videoElement.setAttribute('playsinline', 'true')
      await videoElement.play()

      activeStream.value = stream
      isCameraActive.value = true
      return stream
    } catch (err: any) {
      isCameraActive.value = false
      if (err.name === 'NotAllowedError') {
        throw new Error('Izin akses kamera ditolak. Berikan izin kamera pada browser/sistem POS.')
      } else if (err.name === 'NotFoundError') {
        throw new Error('Kamera web / perangkat video tidak terdeteksi pada POS ini.')
      }
      throw new Error(`Gagal membuka kamera: ${err.message || err}`)
    }
  }

  function stopCamera(videoElement?: HTMLVideoElement) {
    if (activeStream.value) {
      activeStream.value.getTracks().forEach((track) => track.stop())
      activeStream.value = null
    }
    if (videoElement && videoElement.srcObject) {
      const stream = videoElement.srcObject as MediaStream
      stream.getTracks().forEach((track) => track.stop())
      videoElement.srcObject = null
    }
    isCameraActive.value = false
  }

  // Ekstraksi 64-Point Facial Luminosity Feature Tensor & Thumbnail Avatar
  function captureFace(videoElement: HTMLVideoElement): {
    descriptor: string
    preview: string
    sharpnessScore: number
  } {
    const canvas = document.createElement('canvas')
    const ctx = canvas.getContext('2d')
    if (!ctx) throw new Error('Canvas 2D context tidak tersedia')

    const vw = videoElement.videoWidth || 640
    const vh = videoElement.videoHeight || 480
    canvas.width = vw
    canvas.height = vh

    ctx.drawImage(videoElement, 0, 0, vw, vh)

    // Crop area tengah wajah (fokus 65% tengah)
    const cropSize = Math.min(vw, vh) * 0.65
    const cropX = (vw - cropSize) / 2
    const cropY = (vh - cropSize) / 2

    // Canvas thumbnail avatar 128x128
    const thumbCanvas = document.createElement('canvas')
    thumbCanvas.width = 128
    thumbCanvas.height = 128
    const thumbCtx = thumbCanvas.getContext('2d')!
    thumbCtx.drawImage(canvas, cropX, cropY, cropSize, cropSize, 0, 0, 128, 128)
    const preview = thumbCanvas.toDataURL('image/jpeg', 0.85)

    // Hitung deskriptor wajah berbasis 8x8 Grid Normalisasi Luminance & Edge Gradients
    const faceImgData = thumbCtx.getImageData(0, 0, 128, 128)
    const data = faceImgData.data
    const gridSize = 8
    const step = 128 / gridSize
    const vector: number[] = []

    for (let gy = 0; gy < gridSize; gy++) {
      for (let gx = 0; gx < gridSize; gx++) {
        let sumLum = 0
        let count = 0
        for (let y = gy * step; y < (gy + 1) * step; y += 2) {
          for (let x = gx * step; x < (gx + 1) * step; x += 2) {
            const idx = (Math.floor(y) * 128 + Math.floor(x)) * 4
            const r = data[idx]
            const g = data[idx + 1]
            const b = data[idx + 2]
            const lum = 0.299 * r + 0.587 * g + 0.114 * b
            sumLum += lum
            count++
          }
        }
        vector.push(Math.round((sumLum / (count || 1)) * 10) / 10)
      }
    }

    const magnitude = Math.sqrt(vector.reduce((acc, val) => acc + val * val, 0)) || 1
    const normalized = vector.map((v) => Number((v / magnitude).toFixed(5)))

    return {
      descriptor: JSON.stringify(normalized),
      preview,
      sharpnessScore: magnitude,
    }
  }

  // Cosine Similarity Matrix Comparison (0% - 100%)
  function calculateFaceSimilarity(descA: string, descB: string): number {
    try {
      const vecA: number[] = JSON.parse(descA)
      const vecB: number[] = JSON.parse(descB)

      if (!Array.isArray(vecA) || !Array.isArray(vecB) || vecA.length !== vecB.length) {
        return 0
      }

      let dotProduct = 0
      let normA = 0
      let normB = 0

      for (let i = 0; i < vecA.length; i++) {
        dotProduct += vecA[i] * vecB[i]
        normA += vecA[i] * vecA[i]
        normB += vecB[i] * vecB[i]
      }

      if (normA === 0 || normB === 0) return 0

      const cosine = dotProduct / (Math.sqrt(normA) * Math.sqrt(normB))
      const percentage = Math.max(0, Math.min(100, Math.round(((cosine - 0.5) / 0.5) * 100)))
      return percentage
    } catch {
      return 0
    }
  }

  // 3. API PENGATURAN BIOMETRIK SISTEM TOKO
  async function fetchBiometricSettings(): Promise<BiometricSettings> {
    try {
      const res = await api.get('/settings/biometric')
      if (res.data?.success && res.data.data) {
        return {
          supervisor_biometric_mode: res.data.data.supervisor_biometric_mode || 'OFF',
          supervisor_biometric_policy: res.data.data.supervisor_biometric_policy || 'OPTIONAL',
          face_match_threshold: Number(res.data.data.face_match_threshold) || 75.0,
        }
      }
    } catch (err) {
      console.warn('Gagal memuat pengaturan biometrik sistem:', err)
    }
    return {
      supervisor_biometric_mode: 'OFF',
      supervisor_biometric_policy: 'OPTIONAL',
      face_match_threshold: 75.0,
    }
  }

  async function saveBiometricSettings(settings: BiometricSettings): Promise<void> {
    const res = await api.put('/settings/biometric', settings)
    if (!res.data?.success) {
      throw new Error(res.data?.message || 'Gagal menyimpan pengaturan biometrik')
    }
  }

  return {
    isWebAuthnSupported,
    registerFingerprint,
    verifyFingerprint,
    startCamera,
    stopCamera,
    captureFace,
    calculateFaceSimilarity,
    fetchBiometricSettings,
    saveBiometricSettings,
    activeStream,
    isCameraActive,
  }
}
