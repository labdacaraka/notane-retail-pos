import { createRouter, createWebHistory } from 'vue-router'
import ServerHostApp from '@/pages/ServerHostApp.vue'

const routes = [
  {
    path: '/',
    name: 'server-host',
    component: ServerHostApp,
  },
  {
    path: '/server-host',
    redirect: '/',
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/',
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
