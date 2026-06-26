import { useState, useEffect, useCallback } from 'react'

interface Gap { query: string; count: number }
interface StaleMemory { memory_id: string; score: number; days_since_access: number; access_count: number }
interface HealthSnapshot {
  space_id: string; snapshot_date: string; total: number; human_ratio: number; ai_ratio: number; co_ratio: number
  conflict_count: number; avg_trust: number; stale_count: number; orphan_count: number; gap_count: number; health_score: number
}

const API = '/api/v1'
const SPACE_ID = 'sp_default'

export default function HealthDashboard() {
  const [snap, setSnap] = useState<HealthSnapshot | null>(null)
  const [gaps, setGaps] = useState<Gap[]>([])
  const [stale, setStale] = useState<StaleMemory[]>([])
  const [loading, setLoading] = useState(false)

  const fetchData = useCallback(async () => {
    setLoading(true)
    try {
      const r = await fetch(`${API}/v3/health/space/${SPACE_ID}`).then(r => r.json())
      if (r.code === 0 && r.data) setSnap(r.data)
      const g = await fetch(`${API}/v3/health/gaps?space_id=${SPACE_ID}`).then(r => r.json())
      if (g.code === 0) setGaps(g.data || [])
      const s = await fetch(`${API}/v3/health/stale?space_id=${SPACE_ID}`).then(r => r.json())
      if (s.code === 0) setStale(s.data || [])
    } catch { /* ignore */ }
    setLoading(false)
  }, [])

  useEffect(() => { fetchData() }, [fetchData])

  const triggerScan = async () => {
    setLoading(true)
    await fetch(`${API}/v3/health/scan`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ space_id: SPACE_ID }) })
    await fetchData()
  }

  const scoreColor = (s: number) => s >= 80 ? 'text-green-600' : s >= 50 ? 'text-yellow-600' : 'text-red-600'
  const scoreBg = (s: number) => s >= 80 ? 'bg-green-100' : s >= 50 ? 'bg-yellow-100' : 'bg-red-100'

  return (
    <div className="max-w-5xl mx-auto space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Knowledge Health</h1>
          <p className="text-gray-500">Monitor your knowledge base vitality and quality.</p>
        </div>
        <button onClick={triggerScan} disabled={loading}
          className="px-4 py-2 bg-indigo-600 text-white rounded-lg text-sm font-medium hover:bg-indigo-700 disabled:opacity-50">
          {loading ? 'Scanning...' : 'Scan Now'}
        </button>
      </div>

      {snap && (
        <div className={`rounded-2xl p-8 ${scoreBg(snap.health_score)} text-center`}>
          <div className={`text-6xl font-bold ${scoreColor(snap.health_score)}`}>
            {snap.health_score.toFixed(1)}
          </div>
          <div className="text-sm text-gray-600 mt-1">Health Score</div>
          <div className="text-xs text-gray-400 mt-1">Snapshot: {snap.snapshot_date}</div>
        </div>
      )}

      {/* Metric cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { label: 'Activity', value: snap ? ((snap.total - snap.stale_count) / Math.max(snap.total, 1) * 100).toFixed(0) + '%' : '--', sub: 'active memories', color: 'text-blue-600' },
          { label: 'Completeness', value: snap ? (snap.gap_count === 0 ? '100%' : ((1 - snap.gap_count / (snap.gap_count + 10)) * 100).toFixed(0) + '%') : '--', sub: `${snap?.gap_count || 0} gaps`, color: 'text-purple-600' },
          { label: 'Freshness', value: snap ? ((1 - snap.stale_count / Math.max(snap.total, 1)) * 100).toFixed(0) + '%' : '--', sub: `${snap?.stale_count || 0} stale`, color: 'text-green-600' },
          { label: 'Trust', value: snap ? (snap.avg_trust * 100).toFixed(0) + '%' : '--', sub: `${snap?.conflict_count || 0} conflicts`, color: 'text-orange-600' },
        ].map((m, i) => (
          <div key={i} className="bg-white rounded-xl border border-gray-200 p-5">
            <div className="text-xs text-gray-400 uppercase tracking-wider">{m.label}</div>
            <div className={`text-2xl font-bold mt-1 ${m.color}`}>{m.value}</div>
            <div className="text-xs text-gray-400 mt-1">{m.sub}</div>
          </div>
        ))}
      </div>

      {/* Ratio bars */}
      {snap && (
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="text-sm font-semibold text-gray-700 mb-3">Memory Provenance</h3>
          <div className="space-y-2">
            {[
              { label: 'Human', pct: snap.human_ratio * 100, color: 'bg-green-400' },
              { label: 'AI', pct: snap.ai_ratio * 100, color: 'bg-purple-400' },
              { label: 'Co', pct: snap.co_ratio * 100, color: 'bg-blue-400' },
            ].map((b, i) => (
              <div key={i} className="flex items-center gap-3">
                <span className="w-14 text-xs text-gray-500">{b.label}</span>
                <div className="flex-1 h-4 bg-gray-100 rounded-full overflow-hidden">
                  <div className={`h-full ${b.color} rounded-full transition-all`} style={{ width: `${b.pct}%` }} />
                </div>
                <span className="text-xs text-gray-400 w-10 text-right">{b.pct.toFixed(0)}%</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Gaps & Stale */}
      <div className="grid md:grid-cols-2 gap-6">
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="text-sm font-semibold text-gray-700 mb-3">Knowledge Gaps ({gaps.length})</h3>
          {gaps.length === 0 ? <p className="text-xs text-gray-400">No gaps detected. Great!</p> : (
            <ul className="space-y-1 max-h-64 overflow-y-auto">
              {gaps.slice(0, 10).map((g, i) => (
                <li key={i} className="text-xs text-gray-600 flex justify-between">
                  <span className="truncate">{g.query}</span>
                  <span className="text-gray-400 ml-2">{g.count}x</span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="bg-white rounded-xl border border-gray-200 p-5">
          <h3 className="text-sm font-semibold text-gray-700 mb-3">Stale Content ({stale.length})</h3>
          {stale.length === 0 ? <p className="text-xs text-gray-400">All content is fresh!</p> : (
            <ul className="space-y-1 max-h-64 overflow-y-auto">
              {stale.slice(0, 10).map((s, i) => (
                <li key={i} className="text-xs text-gray-600 flex justify-between">
                  <span className="truncate font-mono">{s.memory_id.slice(0, 12)}...</span>
                  <span className="text-gray-400 ml-2">{s.days_since_access}d ago</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
