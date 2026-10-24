/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#f2fdf4',
          100: '#e1fae9',
          200: '#c2f4d3',
          300: '#92eaaf',
          400: '#59d369',
          500: '#25c0a6',
          600: '#149b88',
          700: '#117b6d',
          800: '#116157',
          900: '#115048',
          950: '#062d29',
        },
        notane: {
          lime: '#9beb1c',
          green: '#59d369',
          teal: '#25c0a6',
          cyan: '#17bbb6',
        },
        retail: {
          dark: '#0f172a',
          card: '#1e293b',
          border: '#334155',
          accent: '#25c0a6',
          warning: '#f59e0b',
          danger: '#ef4444',
          info: '#17bbb6',
        }
      },
      fontFamily: {
        mono: ['JetBrains Mono', 'Fira Code', 'Courier New', 'monospace'],
      }
    },
  },
  plugins: [],
}
