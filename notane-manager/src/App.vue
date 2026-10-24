<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from './components/layout/Sidebar.vue'
import Navbar from './components/layout/Navbar.vue'
import { useProviderStore } from './stores/providerStore'
import { useAuthStore } from './stores/authStore'
import { CheckCircle2, AlertCircle, X } from 'lucide-vue-next'

const route = useRoute()
const providerStore = useProviderStore()
const authStore = useAuthStore()

onMounted(async () => {
  if (authStore.isAuthenticated) {
    await providerStore.fetchStatus()
  }
})
</script>

<template>
  <!-- Full Screen Login View -->
  <div v-if="route.name === 'login'" class="h-screen w-screen overflow-hidden bg-slate-950">
    <router-view />
  </div>

  <!-- Authenticated Studio Layout -->
  <div v-else class="flex h-screen w-screen overflow-hidden bg-slate-950 font-sans text-slate-100">
    <!-- Left Navigation Sidebar -->
    <Sidebar />

    <!-- Main Content Area -->
    <div class="flex-1 flex flex-col min-w-0 overflow-hidden">
      <Navbar />

      <!-- Toast / Floating Notification Alert -->
      <div v-if="providerStore.successMessage || providerStore.errorMessage" class="px-6 pt-4">
        <div
          v-if="providerStore.successMessage"
          class="p-3 rounded-xl bg-emerald-500/15 border border-emerald-500/30 text-emerald-300 text-xs flex items-center justify-between shadow-lg"
        >
          <div class="flex items-center gap-2">
            <CheckCircle2 class="w-4 h-4 text-emerald-400 shrink-0" />
            <span>{{ providerStore.successMessage }}</span>
          </div>
          <button @click="providerStore.clearNotifications" class="text-emerald-400 hover:text-emerald-200">
            <X class="w-3.5 h-3.5" />
          </button>
        </div>

        <div
          v-if="providerStore.errorMessage"
          class="p-3 rounded-xl bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs flex items-center justify-between shadow-lg"
        >
          <div class="flex items-center gap-2">
            <AlertCircle class="w-4 h-4 text-rose-400 shrink-0" />
            <span>{{ providerStore.errorMessage }}</span>
          </div>
          <button @click="providerStore.clearNotifications" class="text-rose-400 hover:text-rose-200">
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <!-- Router View Container -->
      <main class="flex-1 overflow-y-auto p-6 min-w-0">
        <router-view />
      </main>
    </div>
  </div>
</template>
