<template>
  <BaseModal
    :title="isFinished ? 'Downloaded' : 'Downloading'"
    @close="checkAndClose"
    content-class="w-full h-[80vh] max-w-screen-md"
    body-class="flex flex-col h-full min-h-0 justify-between gap-6"
  >
    <div class="flex flex-col items-center justify-center gap-1">
      <div class="w-full bg-brave-95 h-1 rounded">
        <div class="bg-brave-30 h-1" :style="{ width: progressWidth }"></div>
      </div>
      <div class="text-[0.7rem] text-brave-30/60 dark:text-brave-95/60 flex gap-3">
        <span>{{ successCount }} FOUND</span>
        <span class="text-yellow-700 dark:text-yellow-400">{{ skippedCount }} SKIPPED</span>
        <span>{{ notFoundCount }} NOT FOUND</span>
        <span class="text-red-800 dark:text-red-400">{{ failureCount }} FAILED</span>
      </div>
    </div>

    <div ref="logRef" class="rounded-lg p-3 bg-brave-98 dark:bg-brave-1 w-full text-xs grow overflow-auto">
      <div :style="{ height: `${logTotalSize}px`, width: '100%', position: 'relative' }">
        <div
          v-for="virtualRow in logVirtualRows"
          :key="virtualRow.index"
          :style="{
            position: 'absolute',
            top: 0,
            left: 0,
            width: '100%',
            height: `${virtualRow.size}px`,
            transform: `translateY(${virtualRow.start}px)`,
          }"
          :class="{
            'text-green-800 dark:text-green-400': log[virtualRow.index].status === 'success',
            'text-yellow-700 dark:text-yellow-400': log[virtualRow.index].status === 'skipped',
            'text-brave-50 dark:text-brave-60': log[virtualRow.index].status === 'not_found',
            'text-red-800 dark:text-red-400': log[virtualRow.index].status === 'failure',
          }"
        >
          <strong>{{ log[virtualRow.index].title }} - {{ log[virtualRow.index].artistName }}</strong>:
          <span>{{ log[virtualRow.index].message }}</span>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex-none flex justify-center gap-2">
        <button v-if="isFinished && failureCount > 0" class="button button-warning px-8 py-2 rounded-full" @click="handleRetry">
          Retry {{ failureCount }} failed
        </button>
        <button v-if="isFinished" class="button button-primary px-8 py-2 rounded-full" @click="checkAndClose">Finish</button>
        <button v-else class="button button-normal px-8 py-2 rounded-full" @click="handleStop">Stop</button>
      </div>
    </template>
  </BaseModal>
</template>

<script setup>
import { onUnmounted, computed, ref } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { useDownloader } from '@/composables/downloader.js'

const {
  downloadQueue,
  downloadProgress,
  successCount,
  skippedCount,
  notFoundCount,
  failureCount,
  totalCount,
  downloadedCount,
  startOver,
  stopDownloading,
  retryFailed,
  log
} = useDownloader()

const emit = defineEmits(['close'])

const logRef = ref(null)

const logVirtualizer = useVirtualizer(
  computed(() => ({
    count: log.value.length,
    getScrollElement: () => logRef.value,
    estimateSize: () => 20,
    overscan: 10,
  }))
)

const logVirtualRows = computed(() => logVirtualizer.value.getVirtualItems())
const logTotalSize = computed(() => logVirtualizer.value.getTotalSize())

const progressWidth = computed(() => {
  if (!downloadQueue.value) {
    return '100%'
  }

  if (downloadProgress.value > 1.0) {
    return '100%'
  }

  return `${downloadProgress.value * 100}%`
})

const isFinished = computed(() => {
  return downloadedCount.value >= totalCount.value
})

const handleRetry = () => {
  retryFailed()
}

const handleStop = () => {
  stopDownloading()
  emit('close')
}

const checkAndClose = () => {
  if (isFinished.value) {
    startOver()
    emit('close')
  } else {
    emit('close')
  }
}

onUnmounted(() => {
  checkAndClose()
})
</script>
