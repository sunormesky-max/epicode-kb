import type { Memory } from '../lib/types'
import ProvenanceBadge from './ProvenanceBadge'
import TrustIndicator from './TrustIndicator'

const reviewStatusColors: Record<string, string> = {
  pending: 'bg-yellow-100 text-yellow-800',
  accepted: 'bg-green-100 text-green-800',
  rejected: 'bg-red-100 text-red-800',
  expired: 'bg-gray-100 text-gray-600',
}

export default function MemoryCard({
  memory,
  highlight,
  score,
  onAdopt,
  onReject,
}: {
  memory: Memory
  highlight?: string
  score?: number
  onAdopt?: (id: string) => void
  onReject?: (id: string) => void
}) {
  const dateStr = new Date(memory.created_at * 1000).toLocaleString()
  const reviewColor = reviewStatusColors[memory.review_status] ?? 'bg-gray-100 text-gray-600'

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4 hover:shadow-md transition-shadow">
      {/* Header row */}
      <div className="flex items-center gap-2 flex-wrap mb-2">
        <ProvenanceBadge provenance={memory.provenance} />
        <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${reviewColor}`}>
          {memory.review_status}
        </span>
        {score !== undefined && (
          <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 text-blue-800 font-medium">
            score: {score.toFixed(3)}
          </span>
        )}
        <span className="text-xs text-gray-400 ml-auto">{dateStr}</span>
      </div>

      {/* Content */}
      <p className="text-sm text-gray-700 mb-3 line-clamp-4">
        {highlight ? (
          <span dangerouslySetInnerHTML={{ __html: highlight }} />
        ) : (
          memory.content.length > 300
            ? `${memory.content.slice(0, 300)}...`
            : memory.content
        )}
      </p>

      {/* Footer */}
      <div className="flex items-center gap-4">
        <div className="flex-1 max-w-[200px]">
          <div className="flex items-center gap-1 mb-0.5">
            <span className="text-xs text-gray-500">Trust</span>
          </div>
          <TrustIndicator trust={memory.trust_level} />
        </div>

        {memory.review_status === 'pending' && onAdopt && onReject && (
          <div className="flex gap-2">
            <button
              onClick={() => onAdopt(memory.id)}
              className="px-3 py-1 text-xs font-medium text-white bg-green-600 rounded hover:bg-green-700 transition-colors"
            >
              Adopt
            </button>
            <button
              onClick={() => onReject(memory.id)}
              className="px-3 py-1 text-xs font-medium text-white bg-red-600 rounded hover:bg-red-700 transition-colors"
            >
              Reject
            </button>
          </div>
        )}
      </div>
    </div>
  )
}
