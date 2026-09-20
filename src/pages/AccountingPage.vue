<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import { TrendingUp } from 'lucide-vue-next'
import { api } from '@/services/api'

const activeTab = ref<'income' | 'journal' | 'coa'>('income')
const incomeStatement = ref<any>(null)
const journalEntries = ref<any[]>([])
const chartOfAccounts = ref<any[]>([])
const isLoading = ref<boolean>(false)

// Pagination states
const jrnPage = ref<number>(1)
const jrnPageSize = ref<number>(10)
const paginatedJournalEntries = computed(() => {
  const start = (jrnPage.value - 1) * jrnPageSize.value
  return journalEntries.value.slice(start, start + jrnPageSize.value)
})

const coaPage = ref<number>(1)
const coaPageSize = ref<number>(10)
const paginatedCOA = computed(() => {
  const start = (coaPage.value - 1) * coaPageSize.value
  return chartOfAccounts.value.slice(start, start + coaPageSize.value)
})

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

async function fetchAccountingData() {
  isLoading.value = true
  try {
    const [incRes, jrnRes, coaRes] = await Promise.all([
      api.get('/reports/income-statement'),
      api.get('/accounting/journal-entries', { params: { limit: 50 } }),
      api.get('/accounting/chart-of-accounts'),
    ])

    if (incRes.data.success) incomeStatement.value = incRes.data.data
    if (jrnRes.data.success) journalEntries.value = jrnRes.data.data
    if (coaRes.data.success) chartOfAccounts.value = coaRes.data.data
  } catch (err) {
    console.error(err)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  fetchAccountingData()
})
</script>

<template>
  <AdminLayout>
    <div class="p-8 space-y-6">
      <!-- Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:accounting-calculator-1" class="w-6 h-6 drop-shadow-xs" />
            <span>Pembukuan Akuntansi & Laporan Keuangan</span>
          </h2>
          <p class="text-xs text-slate-600">Pencatatan akuntansi double-entry otomatis terintegrasi langsung dari operasional gerai.</p>
        </div>

        <!-- Navigation Tabs -->
        <div class="flex items-center gap-1.5 bg-slate-200/80 border border-slate-300 p-1 rounded-lg">
          <button
            @click="activeTab = 'income'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition"
            :class="activeTab === 'income' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            Laba Rugi (Income Statement)
          </button>
          <button
            @click="activeTab = 'journal'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition"
            :class="activeTab === 'journal' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            Buku Jurnal Umum (Ledger)
          </button>
          <button
            @click="activeTab = 'coa'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition"
            :class="activeTab === 'coa' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            Bagan Akun (COA)
          </button>
        </div>
      </div>

      <!-- Tab 1: Income Statement (Laba Rugi) -->
      <div v-if="activeTab === 'income' && incomeStatement" class="space-y-5">
        <!-- Summary Cards Grid -->
        <div class="grid grid-cols-1 md:grid-cols-4 gap-4">
          <div class="bg-white border border-slate-300 p-5 rounded-xl shadow-sm">
            <span class="text-xs font-bold text-slate-500 uppercase">Total Pendapatan Penjualan</span>
            <div class="text-2xl font-bold text-slate-900 font-mono mt-1">
              {{ formatCurrency(incomeStatement.total_revenue) }}
            </div>
          </div>

          <div class="bg-white border border-slate-300 p-5 rounded-xl shadow-sm">
            <span class="text-xs font-bold text-slate-500 uppercase">Beban Pokok Penjualan (HPP)</span>
            <div class="text-2xl font-bold text-rose-700 font-mono mt-1">
              {{ formatCurrency(incomeStatement.total_cogs) }}
            </div>
          </div>

          <div class="bg-white border border-slate-300 p-5 rounded-xl shadow-sm">
            <span class="text-xs font-bold text-slate-500 uppercase">Laba Kotor (Gross Profit)</span>
            <div class="text-2xl font-bold text-sky-700 font-mono mt-1">
              {{ formatCurrency(incomeStatement.gross_profit) }}
            </div>
          </div>

          <div class="bg-white border border-slate-300 p-5 rounded-xl shadow-sm">
            <span class="text-xs font-bold text-slate-500 uppercase">Laba Bersih (Net Income)</span>
            <div class="text-2xl font-bold font-mono mt-1" :class="incomeStatement.net_profit >= 0 ? 'text-emerald-700' : 'text-rose-700'">
              {{ formatCurrency(incomeStatement.net_profit) }}
            </div>
          </div>
        </div>

        <!-- Income Statement Breakdown Report -->
        <div class="bg-white border border-slate-300 rounded-xl p-6 shadow-sm space-y-6">
          <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2 border-b border-slate-200 pb-3">
            <TrendingUp class="w-4 h-4 text-emerald-600" />
            Rincian Laporan Laba Rugi Komprehensif
          </h3>

          <!-- Revenue Section -->
          <div class="space-y-2">
            <div class="text-xs font-bold text-slate-700 uppercase tracking-wider">I. PENDAPATAN OPERASIONAL</div>
            <div
              v-for="rev in incomeStatement.revenue_details"
              :key="rev.code"
              class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200/80 rounded-lg font-mono"
            >
              <span class="text-slate-800 font-sans font-medium">{{ rev.code }} - {{ rev.name }}</span>
              <span class="text-emerald-700 font-bold">{{ formatCurrency(rev.amount) }}</span>
            </div>
          </div>

          <!-- Expenses Section -->
          <div class="space-y-2">
            <div class="text-xs font-bold text-slate-700 uppercase tracking-wider">II. BEBAN OPERASIONAL & HPP</div>
            <div
              v-for="exp in incomeStatement.expense_details"
              :key="exp.code"
              class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200/80 rounded-lg font-mono"
            >
              <span class="text-slate-800 font-sans font-medium">{{ exp.code }} - {{ exp.name }}</span>
              <span class="text-rose-700 font-bold">{{ formatCurrency(exp.amount) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Tab 2: Journal Entries (Buku Jurnal Umum) -->
      <div v-else-if="activeTab === 'journal'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3.5 px-4">No. Jurnal & Waktu</th>
                <th class="py-3.5 px-4">Referensi</th>
                <th class="py-3.5 px-4">Deskripsi Transaksi</th>
                <th class="py-3.5 px-4 text-right">Total Debet</th>
                <th class="py-3.5 px-4 text-right">Total Kredit</th>
                <th class="py-3.5 px-4 text-center">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200">
              <template v-for="jrn in paginatedJournalEntries" :key="jrn.entry.id">
                <tr class="bg-slate-50 font-mono font-bold">
                  <td class="py-3 px-4 text-slate-900">
                    <div>{{ jrn.entry.journal_number }}</div>
                    <div class="text-[10px] text-slate-500 font-normal">{{ new Date(jrn.entry.created_at).toLocaleString('id-ID') }}</div>
                  </td>
                  <td class="py-3 px-4 text-sky-700">{{ jrn.entry.reference_type }}</td>
                  <td class="py-3 px-4 font-sans text-slate-800">{{ jrn.entry.description }}</td>
                  <td class="py-3 px-4 text-right text-emerald-700 font-bold">{{ formatCurrency(jrn.entry.total_debit) }}</td>
                  <td class="py-3 px-4 text-right text-emerald-700 font-bold">{{ formatCurrency(jrn.entry.total_credit) }}</td>
                  <td class="py-3 px-4 text-center">
                    <span class="bg-emerald-50 text-emerald-800 text-[10px] px-2 py-0.5 rounded-full border border-emerald-300 font-bold">
                      Balanced ✓
                    </span>
                  </td>
                </tr>

                <!-- Nested Journal Lines -->
                <tr v-for="line in jrn.lines" :key="line.id" class="bg-white text-[11px] font-mono text-slate-600">
                  <td class="py-1.5 px-4 pl-8" colspan="2">
                    <span class="text-slate-800">{{ line.account_code }} - {{ line.account_name }}</span>
                  </td>
                  <td class="py-1.5 px-4 font-sans italic text-slate-500">{{ line.memo }}</td>
                  <td class="py-1.5 px-4 text-right text-slate-700">{{ line.debit > 0 ? formatCurrency(line.debit) : '-' }}</td>
                  <td class="py-1.5 px-4 text-right text-slate-700">{{ line.credit > 0 ? formatCurrency(line.credit) : '-' }}</td>
                  <td></td>
                </tr>
              </template>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="jrnPage"
            v-model:pageSize="jrnPageSize"
            :totalItems="journalEntries.length"
          />
        </div>
      </div>

      <!-- Tab 3: Chart of Accounts (Bagan Akun) -->
      <div v-else-if="activeTab === 'coa'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3.5 px-4">Kode Akun</th>
                <th class="py-3.5 px-4">Nama Akun</th>
                <th class="py-3.5 px-4">Kategori Akun</th>
                <th class="py-3.5 px-4 text-center">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr v-for="coa in paginatedCOA" :key="coa.id" class="hover:bg-emerald-50/40 transition">
                <td class="py-3.5 px-4 font-bold text-sky-700">{{ coa.code }}</td>
                <td class="py-3.5 px-4 font-sans font-semibold text-slate-900">{{ coa.name }}</td>
                <td class="py-3.5 px-4">
                  <span v-if="coa.code.startsWith('1')" class="text-emerald-700 font-bold text-[11px]">AKTIVA / ASSET</span>
                  <span v-else-if="coa.code.startsWith('2')" class="text-amber-700 font-bold text-[11px]">KEWAJIBAN / LIABILITY</span>
                  <span v-else-if="coa.code.startsWith('3')" class="text-purple-700 font-bold text-[11px]">MODAL / EQUITY</span>
                  <span v-else-if="coa.code.startsWith('4')" class="text-sky-700 font-bold text-[11px]">PENDAPATAN / REVENUE</span>
                  <span v-else-if="coa.code.startsWith('5')" class="text-rose-700 font-bold text-[11px]">BEBAN / EXPENSE</span>
                </td>
                <td class="py-3.5 px-4 text-center">
                  <span class="bg-emerald-50 text-emerald-800 border border-emerald-300 text-[10px] px-2.5 py-0.5 rounded-full font-bold">Aktif</span>
                </td>
              </tr>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="coaPage"
            v-model:pageSize="coaPageSize"
            :totalItems="chartOfAccounts.length"
          />
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
