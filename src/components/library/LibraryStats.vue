<template>
  <div v-if="stats && stats.total > 0" class="flex flex-col gap-1 min-w-[12rem]">
    <div class="w-full h-1.5 rounded flex overflow-hidden bg-brave-90 dark:bg-brave-20">
      <div class="bg-green-500" :style="{ width: pct(stats.synced) }" />
      <div class="bg-blue-400" :style="{ width: pct(stats.plain_only) }" />
      <div class="bg-brave-60" :style="{ width: pct(stats.instrumental) }" />
      <div class="bg-red-400" :style="{ width: pct(stats.missing) }" />
    </div>
    <div class="text-[0.6rem] text-brave-40 dark:text-brave-70 flex gap-2 flex-wrap">
      <span><span class="inline-block w-1.5 h-1.5 rounded-sm bg-green-500 mr-0.5" />{{ stats.synced }} Synced</span>
      <span><span class="inline-block w-1.5 h-1.5 rounded-sm bg-blue-400 mr-0.5" />{{ stats.plain_only }} Plain</span>
      <span><span class="inline-block w-1.5 h-1.5 rounded-sm bg-brave-60 mr-0.5" />{{ stats.instrumental }} Instr.</span>
      <span><span class="inline-block w-1.5 h-1.5 rounded-sm bg-red-400 mr-0.5" />{{ stats.missing }} Missing</span>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

const stats = ref(null)
let unlisten = null

const fetchStats = async () => {
  try {
    stats.value = await invoke('get_library_stats')
  } catch (error) {
    console.error('Failed to fetch library stats:', error)
  }
}

const pct = (count) => {
  if (!stats.value || stats.value.total === 0) return '0%'
  return `${(count / stats.value.total) * 100}%`
}

onMounted(async () => {
  await fetchStats()
  unlisten = await listen('reload-track-id', fetchStats)
})

onUnmounted(() => {
  if (unlisten) unlisten()
})
</script>
