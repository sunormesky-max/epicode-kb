import { useState, useEffect, useCallback } from 'react'

interface ContextItem {
  id: string; content: string; provenance: string; trust_level: number
}

interface SidePanelProps {
  memoryId: string
  editorContent: string
}

export default function SidePanel({ memoryId, editorContent }: SidePanelProps) {
  const [related, setRelated] = useState<ContextItem[]>([])
  const [warnings, setWarnings] = useState<string[]>([])
  const [visible, setVisible] = useState(true)

  const fetchContext = useCallback(async () => {
    if (!editorContent || editorContent.length < 20) return
    try {
      const r = await fetch(
        `/api/v1/collab/context?memory_id=${memoryId}&cursor=${encodeURIComponent(editorContent.slice(-200))}&space_id=sp_default`,
        { headers: authHeaders() },
      ).then((r) => r.json())
      if (r.code === 0 && r.data) {
        setRelated((r.data.related || []).slice(0, 5))
        setWarnings(r.data.warnings || [])
      }
    } catch { /* ignore — panel is non-blocking */ }
  }, [memoryId, editorContent])

  useEffect(() => {
    const timer = setTimeout(fetchContext, 3000)
    return () => clearTimeout(timer)
  }, [fetchContext])

  if (!visible) {
    return (
      <button onClick={() => setVisible(true)}
        className="fixed right-4 top-20 w-10 h-10 bg-indigo-600 text-white rounded-full shadow-lg flex items-center justify-center text-sm hover:bg-indigo-700 z-40">
        AI
      </button>
    )
  }

  return (
    <div className="w-72 bg-white border-l border-gray-200 h-full overflow-y-auto p-4 flex-shrink-0">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-sm font-semibold text-gray-700">AI Assistant</h3>
        <button onClick={() => setVisible(false)} className="text-gray-400 hover:text-gray-600 text-xs">✕</button>
      </div>

      {/* Warnings */}
      {warnings.length > 0 && (
        <div className="mb-4 space-y-1">
          <div className="text-xs font-medium text-amber-600 mb-1">⚠️ Potential Issues</div>
          {warnings.map((w, i) => (
            <div key={i} className="text-xs text-amber-800 bg-amber-50 rounded-lg p-2">{w}</div>
          ))}
        </div>
      )}

      {/* Related knowledge */}
      <div className="mb-4">
        <div className="text-xs font-medium text-gray-500 mb-2">Related Knowledge</div>
        {related.length === 0 ? (
          <p className="text-xs text-gray-400 italic">Start typing to see related knowledge...</p>
        ) : (
          <div className="space-y-2">
            {related.map((item, i) => (
              <div key={i} className="bg-gray-50 rounded-lg p-2">
                <p className="text-xs text-gray-700 line-clamp-3">{item.content.slice(0, 150)}</p>
                <div className="flex items-center gap-2 mt-1">
                  <span className={`text-[10px] px-1.5 py-0.5 rounded ${
                    item.provenance === 'human' ? 'bg-green-100 text-green-600' :
                    item.provenance === 'ai' ? 'bg-purple-100 text-purple-600' : 'bg-gray-100 text-gray-500'
                  }`}>{item.provenance}</span>
                  <span className="text-[10px] text-gray-400">trust: {item.trust_level.toFixed(1)}</span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Quick tips */}
      <div className="border-t border-gray-100 pt-3">
        <div className="text-[10px] text-gray-400">
          <p className="mb-1">💡 AI analyzes your edits and surfaces:</p>
          <ul className="list-disc pl-3 space-y-0.5">
            <li>Existing conclusions on this topic</li>
            <li>Potential contradictions</li>
            <li>Suggested links to related knowledge</li>
          </ul>
        </div>
      </div>
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
