---
trigger: always_on
---

# AI AGENT RULES: TAURI (RUST) + VUE.JS (SECURITY, VALIDATION, & PERMISSION CHECKS)

You are an expert full-stack engineer developing a secure desktop application using Tauri (Rust backend core) and Vue.js (frontend client).

Every code change, feature proposal, or refactoring you generate MUST strictly follow the defense-in-depth principles below.

---

## 1. CORE PRINCIPLES & THREAT MODEL

1. **Zero Trust on Frontend:**
   - The desktop frontend (Vue/WebView) runs in the user's environment. Memory, LocalStorage, and DOM can be inspected or altered.
   - Frontend validation and permission checks are strictly for UX (form feedback, preventing redundant IPC, conditional UI rendering).
   - Rust backend commands (`#[tauri::command]`) are the SINGLE SOURCE OF TRUTH for authorization and data integrity.

2. **Fail-Closed by Default:**
   - All mutations, file operations, native OS APIs, and sensitive database queries are DENIED unless permissions and payload schemas explicitly pass.

3. **Defense-in-Depth:**
   - Form/Input Validation: Client (Zod/VeeValidate) -> Pre-invoke check -> Rust Deserialization -> Rust Semantic Validation (`validator` crate or domain checks).
   - Permission Control: Router Guards -> UI Elements (`v-can` / disabled state) -> Rust Command Auth Guards.

---

## 2. BACKEND RULES (RUST & TAURI)

### A. Tauri Commands & Authorization Pattern
- Every sensitive `#[tauri::command]` MUST inject session/auth state (e.g., `tauri::State<'_, AppState>`) and verify permissions before executing any business logic.
- Commands must never use `.unwrap()` or `.expect()` on untrusted input or runtime operations; return structured `Result<T, AppError>`.
- Use a reusable authorization helper to keep commands clean.

```rust
// Standard Command Template
#[tauri::command]
pub async fn execute_action(
    state: tauri::State<'_, AppState>,
    payload: ActionPayload,
) -> Result<ActionResponse, AppError> {
    // 1. Authentication & Permission Verification (MANDATORY)
    state.auth_guard.require_permission(&state.current_session()?, Permission::CanExecuteAction)?;

    // 2. Structural & Business Validation (MANDATORY)
    payload.validate().map_err(AppError::Validation)?;

    // 3. Safe Domain Execution
    let result = state.domain_service.execute(payload).await?;

    Ok(result)
}
```

### B. Input Validation & Data Boundaries
- Model incoming parameters into strongly typed DTO structs deriving `serde::Deserialize` and `validator::Validate`.
- Enforce constraints:
  - Strings: trimming, length limits, regex for alphanumeric/slugs/emails, sanitization against control characters.
  - Numbers: explicit minimums, maximums, and positive-only where applicable.
  - Paths & Files: NEVER accept raw arbitrary paths without validating canonical paths against allowed whitelisted directories.

```rust
use validator::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct ActionPayload {
    #[validate(length(min = 3, max = 100, message = "Name must be between 3 and 100 characters"))]
    pub name: String,

    #[validate(range(min = 1, max = 1000, message = "Amount must be between 1 and 1000"))]
    pub amount: u32,
}
```

### C. Standard Error Handling
- Define a unified error enum deriving `serde::Serialize` so the Vue frontend can programmatically react to validation and authorization rejections.

```rust
#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation failed")]
    Validation(validator::ValidationErrors),

    #[error("Internal error")]
    Internal(String),
}
```

---

## 3. FRONTEND RULES (VUE.JS + TYPESCRIPT)

### A. Reactive Permission Checking & UI State
- Maintain active user permissions inside a centralized Pinia store (e.g., `useAuthStore`).
- Implement navigation guards in Vue Router to restrict unauthorized page visits.
- Provide a helper function (e.g., `usePermission`) and a custom directive (`v-can`) to disable or hide UI elements.
- Never show active mutation buttons (Save, Delete, Update) if the current user lacks the required permission.

```vue
<!-- Component Implementation Pattern -->
<script setup lang="ts">
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core' // Or '@tauri-apps/api/tauri' depending on Tauri v1/v2
import { useAuthStore } from '@/stores/auth'
import { usePermission } from '@/composables/usePermission'
import { z } from 'zod'

const { hasPermission } = usePermission()
const isSubmitting = ref(false)
const errors = ref<Record<string, string>>({})

// Form Schema
const schema = z.object({
  name: z.string().trim().min(3).max(100),
  amount: z.number().int().min(1).max(1000)
})

const form = ref({ name: '', amount: 1 })

async function handleSubmit() {
  errors.value = {}
  
  // 1. Client-Side Schema Validation
  const validationResult = schema.safeParse(form.value)
  if (!validationResult.success) {
    validationResult.error.errors.forEach(err => {
      const field = err.path[0] as string
      errors.value[field] = err.message
    })
    return
  }

  // 2. Pre-flight Permission Guard
  if (!hasPermission('CanExecuteAction')) {
    errors.value.global = 'You do not have permission to perform this action.'
    return
  }

  isSubmitting.value = true
  try {
    await invoke('execute_action', { payload: validationResult.data })
  } catch (err: any) {
    // 3. Map Backend Error Responses
    handleBackendError(err)
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <form @submit.prevent="handleSubmit">
    <div>
      <input v-model="form.name" :disabled="!hasPermission('CanExecuteAction')" />
      <span v-if="errors.name" class="error">{{ errors.name }}</span>
    </div>

    <!-- UI check prevents invalid/unauthorized invocations -->
    <button 
      type="submit" 
      :disabled="!hasPermission('CanExecuteAction') || isSubmitting"
    >
      {{ isSubmitting ? 'Processing...' : 'Submit' }}
    </button>
  </form>
</template>
```

---

## 4. CODE GENERATION PROTOCOL (FOR THE AI AGENT)

Whenever asked to write, modify, or refactor features:
1. **Always implement both layers:** If creating a new Tauri command, generate both the secured Rust handler and the corresponding Vue composable/form with validation.
2. **Never leave empty stubs:** Do not use `// TODO: add validation here` or `// assume permission is valid`. Provide working validation schemas and permission assertions.
3. **Explicit Typing:** Always define explicit TypeScript interfaces matching the Rust structs for payloads and IPC responses.
4. **No Panic Policy:** In Rust, avoid `.unwrap()` and `.expect()` in command execution paths. Always return `Result<T, E>`.