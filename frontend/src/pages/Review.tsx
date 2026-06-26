import { useState, useEffect, useCallback } from 'react'

interface Proposal {
  id: string; space_id: string; proposal_type: string; source_memory_ids: string[]
  proposed_content: string | null; confidence: number | null; status: string
  created_at: number; ai_model: string | null
}

const API = '/api/v1'
const SPACE_ID = 'sp_default'

const typeIcon: Record<string, string> = { merge: '🔗', link: '🔗', summarize: '📝', conflict: '⚠️', archive: '📦' }
const typeLabel: Record<string, string> = { merge: 'Merge', link: 'Link', summarize: 'Summarize', conflict: 'Conflict', archive: 'Archive' }

export default function Review() {
  const [proposals, setProposals] = useState<Proposal[]>([])
  const [filter, setFilter] = useState('pending')
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(true)
  const [feedback, setFeedback] = useState('')

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const r = await fetch(`${API}/proposals?space_id=${SPACE_ID}&status=${filter}&page=1&limit=50`).then(r => r.json())
      if (r.code === 0) setProposals(r.data || [])
    } catch { /* ignore */ }
    setLoading(false)
  }, [filter])

  useEffect(() => { load() }, [load])

  const act = async (type: 'approve' | 'reject' | 'batch', ids: string[]) => {
    if (type === 'batch') {
      await fetch(`${API}/proposals/batch`, {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action: 'approve', proposal_ids: ids, feedback: feedback || null }),
      })
    } else {
      for (const id of ids) {
        await fetch(`${API}/proposals/${id}/${type}`, {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ feedback: type === 'reject' ? feedback || null : null }),
        })
      }
    }
    setSelected(new Set())
    setFeedback('')
    load()
  }

  const toggleSelect = (id: string) => {
    const next = new Set(selected)
    next.has(id) ? next.delete(id) : next.add(id)
    setSelected(next)
  }

  const selectAll = () => {
    if (selected.size === proposals.length) setSelected(new Set())
    else setSelected(new Set(proposals.map(p => p.id)))
  }

  const scan = async () => {
    setLoading(true)
    await fetch(`${API}/dream/scan`, {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ space_id: SPACE_ID }),
    })
    load()
  }

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Review Queue</h1>
          <p className="text-gray-500">Review and approve AI-generated proposals.</p>
        </div>
        <div className="flex gap-2">
          <button onClick={scan} disabled={loading}
            className="px-3 py-1.5 text-sm bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 disabled:opacity-50">
            Scan Space
          </button>
          {selected.size > 0 && (
            <button onClick={() => act('batch', [...selected])}
              className="px-3 py-1.5 text-sm bg-green-600 text-white rounded-lg hover:bg-green-700">
              Approve ({selected.size})
            </button>
          )}
        </div>
      </div>

      {/* Filter tabs */}
      <div className="flex gap-1 bg-gray-100 rounded-lg p-1 w-fit">
        {['pending', 'approved', 'rejected'].map(f => (
          <button key={f} onClick={() => setFilter(f)}
            className={`px-4 py-1.5 text-sm rounded-md transition-colors ${filter === f ? 'bg-white shadow text-gray-900 font-medium' : 'text-gray-500 hover:text-gray-700'}`}>
            {f.charAt(0).toUpperCase() + f.slice(1)}
          </button>
        ))}
      </div>

      {loading ? (
        <div className="text-center py-12 text-gray-400">Loading proposals...</div>
      ) : proposals.length === 0 ? (
        <div className="bg-white rounded-xl border border-gray-200 p-12 text-center">
          <div className="text-5xl mb-4">📋</div>
          <h2 className="text-lg font-semibold text-gray-700 mb-2">No {filter} Proposals</h2>
          <p className="text-gray-500">Click "Scan Space" to generate AI proposals.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {/* Select all */}
          {filter === 'pending' && proposals.length > 0 && (
            <label className="flex items-center gap-2 px-3 py-1 text-xs text-gray-500 cursor-pointer">
              <input type="checkbox" checked={selected.size === proposals.length} onChange={selectAll} />
              Select All ({proposals.length})
            </label>
          )}

          {proposals.map(p => (
            <div key={p.id}
              className={`bg-white rounded-xl border p-5 transition-colors ${selected.has(p.id) ? 'border-indigo-400 bg-indigo-50' : 'border-gray-200'}`}>
              <div className="flex items-start gap-3">
                {filter === 'pending' && (
                  <input type="checkbox" checked={selected.has(p.id)} onChange={() => toggleSelect(p.id)}
                    className="mt-1.5" />
                )}
                <span className="text-lg">{typeIcon[p.proposal_type] || '📌'}</span>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="text-sm font-medium text-gray-900">{typeLabel[p.proposal_type] || p.proposal_type}</span>
                    {p.confidence !== null && (
                      <span className={`text-xs px-2 py-0.5 rounded-full ${p.confidence >= 0.7 ? 'bg-green-100 text-green-700' : 'bg-yellow-100 text-yellow-700'}`}>
                        {(p.confidence * 100).toFixed(0)}% confidence
                      </span>
                    )}
                    <span className="text-xs text-gray-400 ml-auto">{new Date(p.created_at * 1000).toLocaleDateString()}</span>
                  </div>
                  {p.proposed_content && (
                    <p className="text-sm text-gray-600 line-clamp-3">{p.proposed_content}</p>
                  )}
                  <div className="flex items-center gap-1 mt-2 flex-wrap">
                    {p.source_memory_ids?.slice(0, 4).map((mid: string) => (
                      <span key={mid} className="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded font-mono">
                        {mid.slice(0, 12)}...
                      </span>
                    ))}
                    {(p.source_memory_ids?.length || 0) > 4 && (
                      <span className="text-xs text-gray-400">+{p.source_memory_ids.length - 4} more</span>
                    )}
                  </div>

                  {filter === 'pending' && (
                    <div className="flex gap-2 mt-3 pt-3 border-t border-gray-100">
                      <button onClick={() => act('approve', [p.id])}
                        className="px-3 py-1 text-xs font-medium bg-green-100 text-green-700 rounded-lg hover:bg-green-200">
                        Approve
                      </button>
                      <button onClick={() => act('reject', [p.id])}
                        className="px-3 py-1 text-xs font-medium bg-red-100 text-red-700 rounded-lg hover:bg-red-200">
                        Reject
                      </button>
                      <input
                        className="flex-1 px-2 py-1 text-xs border border-gray-200 rounded-lg"
                        placeholder="Optional feedback..."
                        value={feedback}
                        onChange={e => setFeedback(e.target.value)}
                      />
                    </div>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
