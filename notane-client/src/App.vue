<script setup lang="ts">
import GlobalDialogModal from '@/components/common/GlobalDialogModal.vue'
import ServerRecoveryBanner from '@/components/common/ServerRecoveryBanner.vue'
</script>

<template>
  <div class="min-h-screen bg-slate-100 text-slate-900 font-sans">
    <ServerRecoveryBanner />
    <router-view />
    <GlobalDialogModal />
  </div>
</template>
