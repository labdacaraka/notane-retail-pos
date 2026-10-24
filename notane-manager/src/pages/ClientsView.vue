<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useProviderStore, type Client } from '../stores/providerStore'
import { usePermission } from '../composables/usePermission'
import PaginationBar from '../components/common/PaginationBar.vue'
import {
  Users,
  Plus,
  Trash2,
  Edit2,
  Building,
  Mail,
  Phone,
  MapPin,
  Calendar,
  X,
  Search,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  LayoutGrid,
  Table as TableIcon,
  KeyRound,
  RefreshCw,
} from 'lucide-vue-next'
import { z } from 'zod'

const router = useRouter()
const providerStore = useProviderStore()
const { hasPermission } = usePermission()

// Server-side state
const searchQuery = ref('')
const sortBy = ref<'created_at' | 'client_name' | 'company_name' | 'email'>('created_at')
const sortOrder = ref<'asc' | 'desc'>('desc')
const viewMode = ref<'table' | 'grid'>('table')

// Modal state
const isModalOpen = ref(false)
const isEditing = ref(false)
const editingClientId = ref<string | null>(null)
const isSubmitting = ref(false)
const formErrors = ref<Record<string, string>>({})

const clientSchema = z.object({
  client_name: z.string().trim().min(2, 'Nama klien minimal 2 karakter'),
  company_name: z.string().trim().min(2, 'Nama perusahaan minimal 2 karakter'),
  email: z.string().trim().email('Format email tidak valid'),
  phone: z.string().optional(),
  address: z.string().optional(),
})

const form = ref({
  client_name: '',
  company_name: '',
  email: '',
  phone: '',
  address: '',
})

async function loadData(page?: number, pageSize?: number) {
  await providerStore.fetchClients({
    page: page ?? providerStore.clientsPagination.page,
    page_size: pageSize ?? providerStore.clientsPagination.page_size,
    search: searchQuery.value.trim() || undefined,
    sort_by: sortBy.value,
    sort_order: sortOrder.value,
  })
}

// Debounce search
let searchTimeout: any = null
watch(searchQuery, () => {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    loadData(1)
  }, 350)
})

onMounted(async () => {
  await loadData(1)
})

function handleSort(column: 'created_at' | 'client_name' | 'company_name' | 'email') {
  if (sortBy.value === column) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = column
    sortOrder.value = 'asc'
  }
  loadData(1)
}

function openCreateModal() {
  isEditing.value = false
  editingClientId.value = null
  form.value = {
    client_name: '',
    company_name: '',
    email: '',
    phone: '',
    address: '',
  }
  formErrors.value = {}
  isModalOpen.value = true
}

function openEditModal(client: Client) {
  isEditing.value = true
  editingClientId.value = client.id
  form.value = {
    client_name: client.client_name,
    company_name: client.company_name,
    email: client.email,
    phone: client.phone || '',
    address: client.address || '',
  }
  formErrors.value = {}
  isModalOpen.value = true
}

async function handleSubmit() {
  formErrors.value = {}
  const res = clientSchema.safeParse(form.value)
  if (!res.success) {
    res.error.errors.forEach((err: any) => {
      formErrors.value[err.path[0] as string] = err.message
    })
    return
  }

  isSubmitting.value = true
  let ok = false
  if (isEditing.value && editingClientId.value) {
    ok = await providerStore.updateClient(editingClientId.value, res.data)
  } else {
    ok = await providerStore.saveClient(res.data)
  }
  isSubmitting.value = false

  if (ok) {
    isModalOpen.value = false
    loadData()
  }
}

async function confirmDelete(client: Client) {
  if (
    confirm(
      `Yakin ingin menghapus klien '${client.client_name}' (${client.company_name})? Seluruh lisensi terkait juga akan terhapus.`
    )
  ) {
    await providerStore.deleteClient(client.id)
  }
}

function goToCreateLicense(clientId: string) {
  router.push({ path: '/licenses', query: { client_id: clientId, action: 'new' } })
}
</script>

<template>
  <div class="space-y-5">
    <!-- Header Controls -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
      <div class="flex items-center gap-3 w-full sm:w-auto">
        <!-- Search Input -->
        <div class="relative w-full sm:w-80">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Cari nama, perusahaan, atau email klien..."
            class="w-full pl-9 pr-4 py-2 bg-slate-900 border border-slate-800 rounded-xl text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-brand-500 transition shadow-inner"
          />
        </div>

        <!-- Reload Button -->
        <button
          @click="loadData()"
          :disabled="providerStore.isLoading"
          title="Muat Ulang Data"
          class="p-2 bg-slate-900 border border-slate-800 hover:bg-slate-800 text-slate-300 rounded-xl transition cursor-pointer"
        >
          <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': providerStore.isLoading }" />
        </button>
      </div>

      <div class="flex items-center gap-2 self-end sm:self-auto">
        <!-- View Mode Switcher -->
        <div class="flex items-center bg-slate-900 border border-slate-800 rounded-xl p-1 text-slate-400">
          <button
            @click="viewMode = 'table'"
            :class="viewMode === 'table' ? 'bg-slate-800 text-white font-bold' : 'hover:text-slate-200'"
            class="px-2.5 py-1 rounded-lg text-xs flex items-center gap-1.5 transition cursor-pointer"
          >
            <TableIcon class="w-3.5 h-3.5" />
            <span>Tabel</span>
          </button>
          <button
            @click="viewMode = 'grid'"
            :class="viewMode === 'grid' ? 'bg-slate-800 text-white font-bold' : 'hover:text-slate-200'"
            class="px-2.5 py-1 rounded-lg text-xs flex items-center gap-1.5 transition cursor-pointer"
          >
            <LayoutGrid class="w-3.5 h-3.5" />
            <span>Kartu</span>
          </button>
        </div>

        <!-- Tambah Klien Button -->
        <button
          @click="openCreateModal"
          :disabled="!hasPermission('client:create')"
          class="px-4 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold flex items-center gap-1.5 transition-all shadow-md shadow-brand-600/20 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah Klien Tenant</span>
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="!providerStore.isLoading && providerStore.clientsPagination.total_items === 0"
      class="py-16 text-center rounded-2xl glass-panel border border-slate-800"
    >
      <Users class="w-12 h-12 text-slate-600 mx-auto mb-3" />
      <div class="text-sm font-semibold text-slate-300">Belum Ada Klien Terdaftar</div>
      <div class="text-xs text-slate-500 mt-1">
        {{ searchQuery ? 'Tidak ada data yang cocok dengan pencarian Anda.' : 'Tambahkan data pemilik outlet atau perusahaan penyewa lisensi pertama Anda.' }}
      </div>
      <button
        v-if="!searchQuery"
        @click="openCreateModal"
        class="mt-4 px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Tambah Klien Sekarang
      </button>
      <button
        v-else
        @click="searchQuery = ''; loadData(1)"
        class="mt-4 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Reset Pencarian
      </button>
    </div>

    <!-- TABLE VIEW (SERVER-SIDE SORTING & PAGING) -->
    <div v-else-if="viewMode === 'table'" class="space-y-3">
      <div class="glass-panel border border-slate-800 rounded-2xl overflow-hidden shadow-xl">
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="bg-slate-900/90 text-slate-400 border-b border-slate-800 select-none">
                <th
                  @click="handleSort('client_name')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Nama Klien / PIC</span>
                    <ArrowUp v-if="sortBy === 'client_name' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'client_name' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('company_name')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Perusahaan / Brand</span>
                    <ArrowUp v-if="sortBy === 'company_name' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'company_name' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('email')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Kontak & Email</span>
                    <ArrowUp v-if="sortBy === 'email' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'email' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th class="py-3 px-4 font-semibold text-slate-400">Alamat</th>

                <th
                  @click="handleSort('created_at')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Terdaftar</span>
                    <ArrowUp v-if="sortBy === 'created_at' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'created_at' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th class="py-3 px-4 font-semibold text-right text-slate-400">Aksi</th>
              </tr>
            </thead>

            <tbody class="divide-y divide-slate-800/60">
              <tr
                v-for="c in providerStore.clientsPagination.items"
                :key="c.id"
                class="hover:bg-slate-800/40 transition-colors group"
              >
                <!-- Client Name -->
                <td class="py-3.5 px-4 font-medium text-slate-200">
                  <div class="flex items-center gap-2">
                    <div class="w-7 h-7 rounded-lg bg-brand-500/10 text-brand-400 flex items-center justify-center font-bold text-xs shrink-0">
                      {{ c.client_name.charAt(0).toUpperCase() }}
                    </div>
                    <div>
                      <div class="font-bold text-white">{{ c.client_name }}</div>
                      <div class="text-[10px] font-mono text-slate-500">{{ c.id.slice(0, 8) }}...</div>
                    </div>
                  </div>
                </td>

                <!-- Company -->
                <td class="py-3.5 px-4 text-slate-300">
                  <div class="flex items-center gap-1.5 font-semibold text-brand-400">
                    <Building class="w-3.5 h-3.5 shrink-0" />
                    <span>{{ c.company_name }}</span>
                  </div>
                </td>

                <!-- Contacts -->
                <td class="py-3.5 px-4 text-slate-300">
                  <div class="space-y-0.5">
                    <div class="flex items-center gap-1.5 text-slate-300">
                      <Mail class="w-3 h-3 text-slate-500" />
                      <span class="font-mono text-[11px]">{{ c.email }}</span>
                    </div>
                    <div v-if="c.phone" class="flex items-center gap-1.5 text-slate-400 text-[11px]">
                      <Phone class="w-3 h-3 text-slate-500" />
                      <span>{{ c.phone }}</span>
                    </div>
                  </div>
                </td>

                <!-- Address -->
                <td class="py-3.5 px-4 text-slate-400 max-w-[200px]">
                  <span class="truncate block" :title="c.address || '-'">{{ c.address || '-' }}</span>
                </td>

                <!-- Registered Date -->
                <td class="py-3.5 px-4 font-mono text-[11px] text-slate-400 whitespace-nowrap">
                  {{ new Date(c.created_at).toLocaleDateString('id-ID', { year: 'numeric', month: 'short', day: 'numeric' }) }}
                </td>

                <!-- Actions -->
                <td class="py-3.5 px-4 text-right">
                  <div class="flex items-center justify-end gap-1">
                    <button
                      @click="goToCreateLicense(c.id)"
                      :disabled="!hasPermission('license:create')"
                      title="Terbitkan Lisensi Klien"
                      class="p-1.5 text-slate-400 hover:text-brand-400 hover:bg-brand-500/10 rounded-lg transition cursor-pointer"
                    >
                      <KeyRound class="w-4 h-4" />
                    </button>
                    <button
                      @click="openEditModal(c)"
                      :disabled="!hasPermission('client:update')"
                      title="Edit Klien"
                      class="p-1.5 text-slate-400 hover:text-amber-400 hover:bg-amber-500/10 rounded-lg transition cursor-pointer"
                    >
                      <Edit2 class="w-4 h-4" />
                    </button>
                    <button
                      @click="confirmDelete(c)"
                      :disabled="!hasPermission('client:delete')"
                      title="Hapus Klien"
                      class="p-1.5 text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition cursor-pointer"
                    >
                      <Trash2 class="w-4 h-4" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.clientsPagination.page"
        :page-size="providerStore.clientsPagination.page_size"
        :total-items="providerStore.clientsPagination.total_items"
        :total-pages="providerStore.clientsPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.clientsPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- GRID VIEW (CARDS) -->
    <div v-else class="space-y-4">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="c in providerStore.clientsPagination.items"
          :key="c.id"
          class="p-5 rounded-2xl glass-panel border border-slate-800 hover:border-slate-700 transition-all flex flex-col justify-between"
        >
          <div class="space-y-3">
            <div class="flex items-start justify-between">
              <div>
                <h3 class="font-bold text-sm text-white">{{ c.client_name }}</h3>
                <div class="text-xs text-brand-400 font-medium flex items-center gap-1 mt-0.5">
                  <Building class="w-3.5 h-3.5" />
                  <span>{{ c.company_name }}</span>
                </div>
              </div>
              <div class="flex items-center gap-1">
                <button
                  @click="openEditModal(c)"
                  :disabled="!hasPermission('client:update')"
                  class="p-1.5 text-slate-400 hover:text-amber-400 hover:bg-amber-500/10 rounded-lg transition cursor-pointer"
                  title="Edit Klien"
                >
                  <Edit2 class="w-3.5 h-3.5" />
                </button>
                <button
                  @click="confirmDelete(c)"
                  :disabled="!hasPermission('client:delete')"
                  class="p-1.5 text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition cursor-pointer"
                  title="Hapus Klien"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <div class="space-y-1.5 text-xs text-slate-400 pt-2 border-t border-slate-800/80">
              <div class="flex items-center gap-2">
                <Mail class="w-3.5 h-3.5 text-slate-500" />
                <span class="truncate">{{ c.email }}</span>
              </div>
              <div v-if="c.phone" class="flex items-center gap-2">
                <Phone class="w-3.5 h-3.5 text-slate-500" />
                <span>{{ c.phone }}</span>
              </div>
              <div v-if="c.address" class="flex items-center gap-2">
                <MapPin class="w-3.5 h-3.5 text-slate-500" />
                <span class="truncate">{{ c.address }}</span>
              </div>
            </div>
          </div>

          <div class="mt-4 pt-3 border-t border-slate-800/60 flex items-center justify-between text-[11px] text-slate-500">
            <span class="flex items-center gap-1 font-mono">
              <Calendar class="w-3 h-3" />
              {{ new Date(c.created_at).toLocaleDateString() }}
            </span>
            <button
              @click="goToCreateLicense(c.id)"
              class="text-xs font-semibold text-brand-400 hover:text-brand-300 transition flex items-center gap-1 cursor-pointer"
            >
              <KeyRound class="w-3.5 h-3.5" />
              <span>Lisensi</span>
            </button>
          </div>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.clientsPagination.page"
        :page-size="providerStore.clientsPagination.page_size"
        :total-items="providerStore.clientsPagination.total_items"
        :total-pages="providerStore.clientsPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.clientsPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- Modal Create / Edit Client -->
    <div v-if="isModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm animate-in fade-in">
      <div class="w-full max-w-md rounded-2xl glass-panel border border-slate-800 bg-slate-900 p-6 shadow-2xl space-y-4">
        <div class="flex items-center justify-between pb-3 border-b border-slate-800">
          <div class="font-bold text-sm text-white">
            {{ isEditing ? 'Edit Data Klien Tenant' : 'Tambah Klien Tenant Baru' }}
          </div>
          <button @click="isModalOpen = false" class="text-slate-400 hover:text-white cursor-pointer">
            <X class="w-4 h-4" />
          </button>
        </div>

        <form @submit.prevent="handleSubmit" class="space-y-3">
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Nama Klien / Penanggung Jawab *</label>
            <input
              v-model="form.client_name"
              type="text"
              placeholder="Contoh: Budi Santoso"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            />
            <span v-if="formErrors.client_name" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.client_name }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Nama Usaha / Perusahaan *</label>
            <input
              v-model="form.company_name"
              type="text"
              placeholder="Contoh: PT Mitra Retail Nusantara"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            />
            <span v-if="formErrors.company_name" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.company_name }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Email Klien *</label>
            <input
              v-model="form.email"
              type="email"
              placeholder="budi@mitraretail.com"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            />
            <span v-if="formErrors.email" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.email }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">No. Telepon / WhatsApp</label>
            <input
              v-model="form.phone"
              type="text"
              placeholder="08123456789"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Alamat Kantor / Outlet</label>
            <textarea
              v-model="form.address"
              rows="2"
              placeholder="Jl. Thamrin No. 45, Jakarta"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            ></textarea>
          </div>

          <div class="pt-3 border-t border-slate-800 flex justify-end gap-2">
            <button
              type="button"
              @click="isModalOpen = false"
              class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl text-xs font-semibold cursor-pointer"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="isSubmitting"
              class="px-4 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold cursor-pointer"
            >
              {{ isSubmitting ? 'Menyimpan...' : (isEditing ? 'Simpan Perubahan' : 'Simpan Klien') }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
