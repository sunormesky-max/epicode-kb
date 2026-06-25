import type { Provenance, ReviewStatus } from '../lib/types'

export interface SearchFiltersState {
  mode: 'semantic' | 'fulltext' | 'hybrid'
  minTrust: number
  provenance: Provenance[]
  reviewStatus: ReviewStatus | ''
}

export default function SearchFilters({
  filters,
  onChange,
}: {
  filters: SearchFiltersState
  onChange: (filters: SearchFiltersState) => void
}) {
  const provenanceOptions: { value: Provenance; label: string }[] = [
    { value: 'human', label: '🟢 Human' },
    { value: 'ai', label: '🟣 AI' },
    { value: 'co', label: '🔵 Collab' },
    { value: 'conflict', label: '🔴 Conflict' },
  ]

  const toggleProvenance = (p: Provenance) => {
    const has = filters.provenance.includes(p)
    const next = has
      ? filters.provenance.filter((x) => x !== p)
      : [...filters.provenance, p]
    onChange({ ...filters, provenance: next })
  }

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4 space-y-4">
      {/* Search mode */}
      <div>
        <label className="block text-xs font-medium text-gray-600 mb-1.5">Search Mode</label>
        <div className="flex gap-2">
          {(['hybrid', 'semantic', 'fulltext'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => onChange({ ...filters, mode })}
              className={`px-3 py-1 text-xs rounded-lg font-medium capitalize transition-colors ${
                filters.mode === mode
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              {mode}
            </button>
          ))}
        </div>
      </div>

      {/* Min trust slider */}
      <div>
        <label className="block text-xs font-medium text-gray-600 mb-1.5">
          Min Trust: {filters.minTrust.toFixed(1)}
        </label>
        <input
          type="range"
          min="0"
          max="1"
          step="0.1"
          value={filters.minTrust}
          onChange={(e) => onChange({ ...filters, minTrust: parseFloat(e.target.value) })}
          className="w-full accent-blue-600"
        />
      </div>

      {/* Provenance filter */}
      <div>
        <label className="block text-xs font-medium text-gray-600 mb-1.5">Provenance</label>
        <div className="flex flex-wrap gap-2">
          {provenanceOptions.map((opt) => (
            <button
              key={opt.value}
              onClick={() => toggleProvenance(opt.value)}
              className={`px-2.5 py-1 text-xs rounded-full transition-colors ${
                filters.provenance.includes(opt.value)
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              {opt.label}
            </button>
          ))}
        </div>
      </div>

      {/* Review status */}
      <div>
        <label className="block text-xs font-medium text-gray-600 mb-1.5">Review Status</label>
        <select
          value={filters.reviewStatus}
          onChange={(e) =>
            onChange({
              ...filters,
              reviewStatus: e.target.value as ReviewStatus | '',
            })
          }
          className="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-lg bg-white"
        >
          <option value="">All</option>
          <option value="accepted">Accepted</option>
          <option value="pending">Pending</option>
          <option value="rejected">Rejected</option>
          <option value="expired">Expired</option>
        </select>
      </div>
    </div>
  )
}
