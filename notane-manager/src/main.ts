import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import { vCan } from './composables/usePermission'
import './style.css'

import { useTheme } from './composables/useTheme'

const app = createApp(App)
const pinia = createPinia()

useTheme().initTheme()

app.use(pinia)
app.use(router)
app.directive('can', vCan)

app.mount('#app')
