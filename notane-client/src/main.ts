import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { Icon, addCollection } from '@iconify/vue'
import streamlineIcons from '@iconify-json/streamline-ultimate-color/icons.json'
import flatColorIcons from '@iconify-json/flat-color-icons/icons.json'
import fluentColorIcons from '@iconify-json/fluent-color/icons.json'
import router from './router'
import App from './App.vue'
import './assets/main.css'

// Register colorful icon sets offline
addCollection(streamlineIcons)
addCollection(flatColorIcons)
addCollection(fluentColorIcons)

const app = createApp(App)
const pinia = createPinia()

app.component('Icon', Icon)
app.use(pinia)
app.use(router)

app.mount('#app')

