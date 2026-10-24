import { useAuthStore } from '../stores/authStore'
import type { Directive } from 'vue'

export function usePermission() {
  const authStore = useAuthStore()

  function hasPermission(permissionSlug: string): boolean {
    return authStore.hasPermission(permissionSlug)
  }

  return {
    hasPermission,
  }
}

// v-can directive: hides or disables element if current user lacks required permission
export const vCan: Directive = {
  mounted(el, binding) {
    const authStore = useAuthStore()
    const requiredPermission = binding.value

    if (!authStore.hasPermission(requiredPermission)) {
      if (binding.modifiers.disable) {
        el.disabled = true
        el.classList.add('opacity-50', 'cursor-not-allowed', 'pointer-events-none')
      } else {
        el.parentNode?.removeChild(el)
      }
    }
  },
  updated(el, binding) {
    const authStore = useAuthStore()
    const requiredPermission = binding.value

    if (!authStore.hasPermission(requiredPermission)) {
      if (binding.modifiers.disable) {
        el.disabled = true
        el.classList.add('opacity-50', 'cursor-not-allowed', 'pointer-events-none')
      } else {
        el.parentNode?.removeChild(el)
      }
    }
  },
}
