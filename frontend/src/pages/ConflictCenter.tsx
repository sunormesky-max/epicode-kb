import { useState, useEffect } from 'react'
import { listConflicts, resolveConflict } from '../lib/api'
import type { Conflict, ConflictResolution } from '../lib/types'

export default function ConflictCenter() {
  const [conflicts, setConflicts] = useState<Conflict[]>([])
  const [selected, setSelected] = useState<Conflict | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadConflicts()
  }, [])

  const loadConflicts = async () => {
    setLoading(true)
    try {
      const data = await listConflicts('sp_default')
      setConflicts(data)
    } catch {
      /* ignore — empty state shown */
    }
    setLoading(false)
  }

  const resolve = async (id: string, resolution: ConflictResolution) => {
    try {
      await resolveConflict(id, resolution)
      setConflicts((cs) => cs.filter((c) => c.id !== id))
      setSelected(null)
    } catch {
      /* ignore */
    }
  }

  if (loading) return <div className="text-center py-12 text-gray-400">Loading conflicts...</div>

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Conflict Center</h1>
        <p className="text-gray-500">Unresolved knowledge contradictions ({conflicts.length})</p>
      </div>

      {conflicts.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
          <div className="text-5xl mb-4">✨</div>
          <h2 className="text-lg font-semibold text-gray-700 mb-2">No Active Conflicts</h2>
          <p className="text-gray-500">Your knowledge base is consistent. Well done!</p>
        </div>
      ) : (
        <div className="grid gap-4">
          {conflicts.map((c) => (
            <div
              key={c.id}
              className="bg-white rounded-xl border border-gray-200 p-5 cursor-pointer hover:border-amber-300 transition-colors"
              onClick={() => setSelected(selected?.id === c.id ? null : c)}
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="text-lg">⚠️</span>
                <span className="text-sm font-medium text-amber-700">Knowledge Conflict</span>
                <span className="text-xs text-gray-400 ml-auto">
                  {new Date(c.created_at * 1000).toLocaleDateString()}
                </span>
              </div>
              {c.content && (
                <p className="text-sm text-gray-600 line-clamp-2">{c.content.slice(0, 200)}</p>
              )}

              {selected?.id === c.id && (
                <div className="mt-4 pt-4 border-t border-gray-100">
                  <div className="grid md:grid-cols-2 gap-4 mb-4">
                    <div className="p-3 bg-gray-50 rounded-lg">
                      <div className="text-xs text-gray-400 mb-1">Statement A</div>
                      <p className="text-sm text-gray-700">
                        {c.conflicting_content_a || c.content}
                      </p>
                    </div>
                    <div className="p-3 bg-gray-50 rounded-lg">
                      <div className="text-xs text-gray-400 mb-1">Statement B</div>
                      <p className="text-sm text-gray-700">
                        {c.conflicting_content_b || '(see memory)'}
                      </p>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        resolve(c.id, 'accept_a')
                      }}
                      className="px-3 py-1.5 text-xs font-medium bg-green-100 text-green-700 rounded-lg hover:bg-green-200"
                    >
                      Accept A
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        resolve(c.id, 'accept_b')
                      }}
                      className="px-3 py-1.5 text-xs font-medium bg-blue-100 text-blue-700 rounded-lg hover:bg-blue-200"
                    >
                      Accept B
                    </button>
                    <button
                      onClick={(e) => {
                        e.stopPropagation()
                        resolve(c.id, 'both_true')
                      }}
                      className="px-3 py-1.5 text-xs font-medium bg-amber-100 text-amber-700 rounded-lg hover:bg-amber-200"
                    >
                      Both True
                    </button>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
