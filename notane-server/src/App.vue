<script setup lang="ts">
import { onMounted } from 'vue'
import GlobalDialogModal from '@/components/common/GlobalDialogModal.vue'
import { useLicenseStore } from '@/stores/licenseStore'

const licenseStore = useLicenseStore()

onMounted(async () => {
  await licenseStore.fetchVersionInfo()
})
</script>

<template>
  <div class="min-h-screen bg-slate-100 text-slate-900 font-sans">
    <router-view />
    <GlobalDialogModal />
  </div>
</template>
