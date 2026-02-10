<template>
  <div ref="parentRef" class="p-4 overflow-y-auto h-full" v-show="props.isActive">
    <div class="mb-2 relative w-[16rem]">
      <input
        v-model="searchInput"
        type="text"
        class="h-8 input px-[2rem] py-1.5 pr-8 w-full dark:text-brave-95"
        placeholder="Search artists..."
      >
      <div class="absolute top-0 left-0 w-[2rem] h-full flex justify-center items-center pl-0.5">
        <Magnify class="text-brave-30 dark:text-brave-95" />
      </div>
      <button
        v-if="searchInput !== ''"
        @click="searchInput = ''"
        class="absolute top-0 right-0 h-full w-8 flex justify-center items-center text-brave-30 hover:text-brave-20 dark:text-brave-95 dark:hover:text-brave-90"
      >
        <Close />
      </button>
    </div>
    <div
      :style="{ height: `${totalSize}px`, width: '100%', position: 'relative' }"
    >
      <div class="w-full">
        <div class="w-full flex">
          <div class="text-xs text-brave-30/70 font-bold flex w-full dark:text-brave-95">
            <div class="text-left flex-none w-[65%] p-1">Artist</div>
            <div class="text-right flex-none w-[15%] p-1"></div>
          </div>
        </div>
        <div class="w-full flex flex-col">
          <div
            v-for="virtualRow in virtualRows"
            :key="virtualRow.index"
            class="group flex flex-col w-full absolute top-0 left-0"
            :style="{
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }"
          >
            <ArtistItem
              :artistId="virtualRow.key"
              @open-artist="openArtist"
            />
          </div>
        </div>
      </div>
    </div>

    <Transition name="slide-fade">
      <ArtistTrackList v-if="currentArtist" :artist="currentArtist" @back="currentArtist = null" />
    </Transition>
  </div>
</template>

<script setup>
import { DownloadMultiple, Magnify, Close } from 'mdue'
import { ref, computed, onMounted, watch } from 'vue'
import { useVirtualizer } from '@tanstack/vue-virtual'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import ArtistItem from './artist-list/ArtistItem.vue'
import ArtistTrackList from './artist-list/ArtistTrackList.vue'
import _debounce from 'lodash/debounce'

const props = defineProps(['isActive'])

const artistIds = ref([])
const parentRef = ref(null)
const currentArtist = ref(null)
const searchInput = ref('')

const fetchArtistIds = async () => {
  artistIds.value = await invoke('get_artist_ids', {
    searchQuery: searchInput.value || null,
  })
}

const debouncedFetch = _debounce(fetchArtistIds, 200)

const rowVirtualizer = useVirtualizer(
  computed(() => ({
    count: artistIds.value.length,
    getScrollElement: () => parentRef.value,
    estimateSize: () => 52,
    overscan: 5,
    paddingStart: 32,
    getItemKey: (index) => artistIds.value[index]
  }))
)

const virtualRows = computed(() => rowVirtualizer.value.getVirtualItems())

const totalSize = computed(() => rowVirtualizer.value.getTotalSize())

const openArtist = async (artist) => {
  currentArtist.value = artist
}

onMounted(async () => {
  if (props.isActive) {
    await fetchArtistIds()
  }
})

watch(() => props.isActive, async () => {
  if (props.isActive) {
    await fetchArtistIds()
  }
})

watch(searchInput, debouncedFetch)
</script>
