/**
 * Audio feedback service for Notane Retail POS
 * Uses the Web Audio API for high-fidelity, zero-latency synthesizer sounds
 * No external audio files required - works completely offline and reliable in Tauri & Web
 */

class SoundService {
  private audioCtx: AudioContext | null = null
  private enabled: boolean = true
  private volume: number = 0.8

  constructor() {
    // Load preference from localStorage
    const savedEnabled = localStorage.getItem('notane_pos_sound_enabled')
    if (savedEnabled !== null) {
      this.enabled = savedEnabled === 'true'
    }

    const savedVolume = localStorage.getItem('notane_pos_sound_volume')
    if (savedVolume !== null) {
      const parsed = parseFloat(savedVolume)
      if (!isNaN(parsed) && parsed >= 0 && parsed <= 1) {
        this.volume = parsed
      }
    }
  }

  private getAudioContext(): AudioContext | null {
    if (typeof window === 'undefined') return null

    if (!this.audioCtx) {
      const AudioContextClass = window.AudioContext || (window as any).webkitAudioContext
      if (AudioContextClass) {
        this.audioCtx = new AudioContextClass()
      }
    }

    // Browser autoplay policy might put AudioContext in suspended state until user gesture
    if (this.audioCtx && this.audioCtx.state === 'suspended') {
      this.audioCtx.resume().catch(() => {})
    }

    return this.audioCtx
  }

  public isEnabled(): boolean {
    return this.enabled
  }

  public setEnabled(val: boolean) {
    this.enabled = val
    localStorage.setItem('notane_pos_sound_enabled', String(val))
  }

  public getVolume(): number {
    return this.volume
  }

  public setVolume(val: number) {
    this.volume = Math.max(0, Math.min(1, val))
    localStorage.setItem('notane_pos_sound_volume', String(this.volume))
  }

  /**
   * Sound 1: Peringatan Stok Habis / Melebihi Stok (Out of Stock / Max Stock Warning)
   * Nada peringatan ganda (dual-tone alert) tegas & jelas bagi kasir
   */
  public playOutOfStock() {
    if (!this.enabled) return
    const ctx = this.getAudioContext()
    if (!ctx) return

    const now = ctx.currentTime

    // Tone 1: 340 Hz (Sawtooth/Triangle blend)
    const osc1 = ctx.createOscillator()
    const gain1 = ctx.createGain()
    osc1.type = 'sawtooth'
    osc1.frequency.setValueAtTime(340, now)
    osc1.frequency.exponentialRampToValueAtTime(220, now + 0.18)

    gain1.gain.setValueAtTime(this.volume * 0.4, now)
    gain1.gain.exponentialRampToValueAtTime(0.001, now + 0.18)

    osc1.connect(gain1)
    gain1.connect(ctx.destination)

    osc1.start(now)
    osc1.stop(now + 0.18)

    // Tone 2: Secondary buzz pulse after 120ms
    const osc2 = ctx.createOscillator()
    const gain2 = ctx.createGain()
    osc2.type = 'sawtooth'
    osc2.frequency.setValueAtTime(260, now + 0.12)
    osc2.frequency.exponentialRampToValueAtTime(160, now + 0.32)

    gain2.gain.setValueAtTime(0, now)
    gain2.gain.setValueAtTime(this.volume * 0.45, now + 0.12)
    gain2.gain.exponentialRampToValueAtTime(0.001, now + 0.32)

    osc2.connect(gain2)
    gain2.connect(ctx.destination)

    osc2.start(now + 0.12)
    osc2.stop(now + 0.32)
  }

  /**
   * Sound 2: Barcode Scan Sukses (Crisp Retail Scanner Beep)
   * Nada 'pip' tajam dan jernih standar pemindai barcode supermarket
   */
  public playScanSuccess() {
    if (!this.enabled) return
    const ctx = this.getAudioContext()
    if (!ctx) return

    const now = ctx.currentTime
    const osc = ctx.createOscillator()
    const gain = ctx.createGain()

    osc.type = 'sine'
    osc.frequency.setValueAtTime(1850, now)
    osc.frequency.exponentialRampToValueAtTime(1920, now + 0.06)

    gain.gain.setValueAtTime(this.volume * 0.35, now)
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.07)

    osc.connect(gain)
    gain.connect(ctx.destination)

    osc.start(now)
    osc.stop(now + 0.07)
  }

  /**
   * Sound 3: Kesalahan / Item Tidak Ditemukan / Validasi Gagal (Error Buzz)
   * Dua ketukan rendah menandakan input salah
   */
  public playError() {
    if (!this.enabled) return
    const ctx = this.getAudioContext()
    if (!ctx) return

    const now = ctx.currentTime

    // Double buzz
    ;[0, 0.11].forEach((delay) => {
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()

      osc.type = 'square'
      osc.frequency.setValueAtTime(180, now + delay)

      gain.gain.setValueAtTime(this.volume * 0.25, now + delay)
      gain.gain.exponentialRampToValueAtTime(0.001, now + delay + 0.09)

      osc.connect(gain)
      gain.connect(ctx.destination)

      osc.start(now + delay)
      osc.stop(now + delay + 0.09)
    })
  }

  /**
   * Sound 4: Pembayaran Berhasil / Checkout Sukses (Payment Success Chime)
   * Akord melodi manis C5 -> E5 -> G5 -> C6 arpeggio khas mesin kasir modern
   */
  public playPaymentSuccess() {
    if (!this.enabled) return
    const ctx = this.getAudioContext()
    if (!ctx) return

    const now = ctx.currentTime
    const notes = [
      { freq: 523.25, time: 0, dur: 0.15 },    // C5
      { freq: 659.25, time: 0.08, dur: 0.15 }, // E5
      { freq: 783.99, time: 0.16, dur: 0.18 }, // G5
      { freq: 1046.50, time: 0.24, dur: 0.45 },// C6 (sustain)
    ]

    notes.forEach((note) => {
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()

      osc.type = 'triangle'
      osc.frequency.setValueAtTime(note.freq, now + note.time)

      gain.gain.setValueAtTime(this.volume * 0.4, now + note.time)
      gain.gain.exponentialRampToValueAtTime(0.001, now + note.time + note.dur)

      osc.connect(gain)
      gain.connect(ctx.destination)

      osc.start(now + note.time)
      osc.stop(now + note.time + note.dur)
    })
  }

  /**
   * Sound 5: Hapus Item / Kosongkan Keranjang (Trash / Remove Click)
   * Nada turun halus untuk konfirmasi penghapusan baris
   */
  public playRemoveItem() {
    if (!this.enabled) return
    const ctx = this.getAudioContext()
    if (!ctx) return

    const now = ctx.currentTime
    const osc = ctx.createOscillator()
    const gain = ctx.createGain()

    osc.type = 'sine'
    osc.frequency.setValueAtTime(440, now)
    osc.frequency.exponentialRampToValueAtTime(220, now + 0.08)

    gain.gain.setValueAtTime(this.volume * 0.25, now)
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.08)

    osc.connect(gain)
    gain.connect(ctx.destination)

    osc.start(now)
    osc.stop(now + 0.08)
  }

  /**
   * Sound 6: Penahanan / Pemanggilan Antrean Nota (Hold / Recall Sound)
   */
  public playHoldRecall() {
    if (!this.enabled) return
    const ctx = this.getAudioContext()
    if (!ctx) return

    const now = ctx.currentTime
    const osc = ctx.createOscillator()
    const gain = ctx.createGain()

    osc.type = 'sine'
    osc.frequency.setValueAtTime(600, now)
    osc.frequency.exponentialRampToValueAtTime(800, now + 0.1)

    gain.gain.setValueAtTime(this.volume * 0.2, now)
    gain.gain.exponentialRampToValueAtTime(0.001, now + 0.1)

    osc.connect(gain)
    gain.connect(ctx.destination)

    osc.start(now)
    osc.stop(now + 0.1)
  }
}

export const soundService = new SoundService()
