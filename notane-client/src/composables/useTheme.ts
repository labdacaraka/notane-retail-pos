import { ref, computed } from 'vue'

export type ThemeMode = 'light' | 'dark'

const THEME_STORAGE_KEY = 'notane_theme'

// Default mode adalah 'light'
const currentTheme = ref<ThemeMode>(
  (localStorage.getItem(THEME_STORAGE_KEY) as ThemeMode) || 'light'
)

export function useTheme() {
  const isDark = computed(() => currentTheme.value === 'dark')

  const applyTheme = (mode: ThemeMode) => {
    currentTheme.value = mode
    localStorage.setItem(THEME_STORAGE_KEY, mode)

    const root = document.documentElement
    if (mode === 'dark') {
      root.classList.add('dark')
      root.classList.remove('light')
      root.setAttribute('data-theme', 'dark')
    } else {
      root.classList.remove('dark')
      root.classList.add('light')
      root.setAttribute('data-theme', 'light')
    }
  }

  const toggleTheme = () => {
    applyTheme(currentTheme.value === 'dark' ? 'light' : 'dark')
  }

  const initTheme = () => {
    const saved = localStorage.getItem(THEME_STORAGE_KEY) as ThemeMode | null
    applyTheme(saved === 'dark' ? 'dark' : 'light')
  }

  return {
    theme: currentTheme,
    isDark,
    toggleTheme,
    setTheme: applyTheme,
    initTheme,
  }
}
