import { useState, useCallback } from 'react'
import MemoryCard from '../components/MemoryCard'
import SearchFilters, { type SearchFiltersState } from '../components/SearchFilters'
import { search, adoptMemory, rejectMemory, ApiError } from '../lib/api'
import type { SearchResult, Provenance } from '../lib/types'

const DEFAULT_SPACE = 'sp_default'

export default function Search() {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResult[]>([])
  const [total, setTotal] = useState(0)
  const [queryTime, setQueryTime] = useState(0)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [hasSearched, setHasSearched] = useState(false)

  const [filters, setFilters] = useState<SearchFiltersState>({
    mode: 'hybrid',
    minTrust: 0,
    provenance: [],
    reviewStatus: '',
  })

  const doSearch = useCallback(async () => {
    if (!query.trim()) return
    setLoading(true)
    setError(null)
    try {
      const params: Record<string, unknown> = {
        q: query,
        space_id: DEFAULT_SPACE,
        mode: filters.mode,
        limit: 20,
      }
      if (filters.minTrust > 0) params.min_trust = filters.minTrust
      if (filters.provenance.length > 0)
        params.provenance = filters.provenance.join(',')
      if (filters.reviewStatus) params.review_status = filters.reviewStatus

      const res = await search(params as Parameters<typeof search>[0])
      setResults(res.results)
      setTotal(res.total)
      setQueryTime(res.query_time_ms)
      setHasSearched(true)
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e))
    } finally {
      setLoading(false)
    }
  }, [query, filters])

  const handleAdopt = async (id: string) => {
    try {
      await adoptMemory(id)
      setResults((prev) =>
        prev.filter((r) => r.memory.id !== id),
      )
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e))
    }
  }

  const handleReject = async (id: string) => {
    try {
      await rejectMemory(id)
      setResults((prev) => prev.filter((r) => r.memory.id !== id),
      )
    } catch (e) {
      setError(e instanceof ApiError ? e.message : String(e))
    }
  }

  return (
    <div className="max-w-5xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-900 mb-1">Search Memories</h1>
      <p className="text-gray-500 mb-6">Hybrid search across semantic and full-text indexes.</p>

      {/* Search bar */}
      <div className="flex gap-2 mb-4">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && doSearch()}
          placeholder="Search for memories..."
          className="flex-1 px-4 py-2.5 border border-gray-300 rounded-lg text-sm focus:ring-2 focus:ring-blue-500 focus:border-transparent"
        />
        <button
          onClick={doSearch}
          disabled={loading || !query.trim()}
          className="px-6 py-2.5 bg-blue-600 text-white rounded-lg font-medium text-sm hover:bg-blue-700 disabled:opacity-50 transition-colors"
        >
          {loading ? 'Searching...' : 'Search'}
        </button>
      </div>

      <div className="flex gap-6">
        {/* Filters sidebar */}
        <div className="w-64 flex-shrink-0">
          <SearchFilters filters={filters} onChange={setFilters} />
        </div>

        {/* Results */}
        <div className="flex-1 min-w-0">
          {error && (
            <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg">
              <p className="text-sm text-red-600">{error}</p>
            </div>
          )}

          {hasSearched && !loading && results.length === 0 && !error && (
            <div className="text-center py-12 text-gray-400">
              <p className="text-lg">No results found</p>
              <p className="text-sm mt-1">Try different keywords or adjust filters.</p>
            </div>
          )}

          {hasSearched && results.length > 0 && (
            <>
              <div className="text-sm text-gray-500 mb-3">
                {total} results in {queryTime}ms
              </div>
              <div className="space-y-3">
                {results.map((r) => (
                  <MemoryCard
                    key={r.memory.id}
                    memory={r.memory}
                    highlight={r.highlight ?? undefined}
                    score={r.score}
                    onAdopt={handleAdopt}
                    onReject={handleReject}
                  />
                ))}
              </div>
            </>
          )}

          {!hasSearched && !loading && (
            <div className="text-center py-12 text-gray-400">
              <p className="text-lg">Start searching</p>
              <p className="text-sm mt-1">Enter a query and press Search.</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

// Cast helper to satisfy TypeScript
type _Provenance = Provenance
