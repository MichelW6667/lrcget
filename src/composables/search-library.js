import { ref } from 'vue'

const searchValue = ref("")
const filters = ref({
  syncedLyricsTracks: true,
  plainLyricsTracks: true,
  instrumentalTracks: true,
  noLyricsTracks: true,
})
const sortBy = ref("title")
const sortOrder = ref("asc")

export function useSearchLibrary() {
  const setSearch = (text, newFilters) => {
    searchValue.value = text
    filters.value = newFilters
  }

  return {
    searchValue,
    filters,
    setSearch,
    sortBy,
    sortOrder,
  }
}
