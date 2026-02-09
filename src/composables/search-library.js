import { ref } from 'vue'

const searchValue = ref("")
const filters = ref({
  syncedLyricsTracks: true,
  plainLyricsTracks: true,
  instrumentalTracks: true,
  noLyricsTracks: true,
})

export function useSearchLibrary() {
  const setSearch = (text, newFilters) => {
    searchValue.value = text
    filters.value = newFilters
  }

  return {
    searchValue,
    filters,
    setSearch,
  }
}
