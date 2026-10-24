<script setup lang="ts">
import GlobalDialogModal from '@/components/common/GlobalDialogModal.vue'
</script>

<template>
  <div class="min-h-screen bg-slate-100 text-slate-900 font-sans">
    <router-view />
    <GlobalDialogModal />
  </div>
</template>
