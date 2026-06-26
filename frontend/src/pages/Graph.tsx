import { useEffect, useState, useCallback, useMemo } from 'react'
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  type Node,
  type Edge,
  type NodeMouseHandler,
  Handle,
  Position,
} from 'reactflow'
import 'reactflow/dist/style.css'

interface GraphNodeData {
  id: string
  label: string
  provenance: string
  trust_level: number
  [key: string]: unknown
}

interface GraphEdgeData {
  source: string
  target: string
  type: 'conflict' | 'similar'
  confidence?: number
}

interface GraphData {
  nodes: GraphNodeData[]
  edges: GraphEdgeData[]
}

const API = '/api/v1'

/** Custom node rendering memory title + provenance badge. */
function MemoryNode({ data }: { data: GraphNodeData }) {
  const provColor =
    data.provenance === 'human'
      ? 'bg-green-100 text-green-700'
      : data.provenance === 'ai'
        ? 'bg-purple-100 text-purple-700'
        : data.provenance === 'co'
          ? 'bg-blue-100 text-blue-700'
          : 'bg-gray-100 text-gray-600'
  return (
    <div className="px-3 py-2 rounded-lg bg-white border border-gray-300 shadow-sm max-w-[180px]">
      <Handle type="target" position={Position.Top} className="!bg-gray-400 !w-2 !h-2" />
      <div className="flex items-center gap-1.5 mb-1">
        <span className={`text-[9px] px-1.5 py-0.5 rounded font-medium ${provColor}`}>
          {data.provenance}
        </span>
        <span className="text-[9px] text-gray-400">trust {data.trust_level.toFixed(1)}</span>
      </div>
      <div className="text-xs text-gray-800 line-clamp-2 leading-tight">{data.label}</div>
      <Handle type="source" position={Position.Bottom} className="!bg-gray-400 !w-2 !h-2" />
    </div>
  )
}

const nodeTypes = { memory: MemoryNode }

export default function Graph() {
  const [data, setData] = useState<GraphData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [highlightEdge, setHighlightEdge] = useState<Set<string>>(new Set())

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const r = await fetch(`${API}/graph?space_id=sp_default`, { headers: authHeaders() }).then((r) => r.json())
      if (r.code === 0 && r.data) {
        setData(r.data)
      } else {
        setError(r.message || 'failed to load graph')
      }
    } catch {
      setError('network error')
    }
    setLoading(false)
  }, [])

  useEffect(() => {
    load()
  }, [load])

  const { nodes, edges } = useMemo(() => {
    if (!data) return { nodes: [] as Node[], edges: [] as Edge[] }
    const ringCount: Record<string, number> = {}
    data.edges.forEach((e) => {
      ringCount[e.source] = (ringCount[e.source] || 0) + 1
      ringCount[e.target] = (ringCount[e.target] || 0) + 1
    })

    const nodes: Node[] = data.nodes.map((n, i) => {
      const angle = (i / Math.max(data.nodes.length, 1)) * 2 * Math.PI
      const radius = 220
      return {
        id: n.id,
        type: 'memory',
        position: {
          x: 400 + radius * Math.cos(angle),
          y: 300 + radius * Math.sin(angle),
        },
        data: n,
      }
    })

    const edges: Edge[] = data.edges.map((e, i) => {
      const isConflict = e.type === 'conflict'
      return {
        id: `e-${i}`,
        source: e.source,
        target: e.target,
        label: isConflict ? '⚠ conflict' : undefined,
        animated: isConflict,
        style: {
          stroke: isConflict ? '#dc2626' : '#9ca3af',
          strokeWidth: isConflict ? 2.5 : 1.5,
          strokeDasharray: isConflict ? '6 4' : undefined,
        },
        data: e,
      }
    })

    return { nodes, edges }
  }, [data])

  const onNodeClick: NodeMouseHandler = useCallback((_, node) => {
    // Highlight all edges touching the clicked node.
    const connected = new Set<string>()
    edges.forEach((edge, idx) => {
      if (edge.source === node.id || edge.target === node.id) {
        connected.add(`e-${idx}`)
      }
    })
    setHighlightEdge(connected)
  }, [edges])

  const styledEdges = useMemo(
    () =>
      edges.map((e) => ({
        ...e,
        style: {
          ...e.style,
          opacity: highlightEdge.size === 0 || highlightEdge.has(e.id) ? 1 : 0.15,
        },
      })),
    [edges, highlightEdge],
  )

  const conflictCount = data?.edges.filter((e) => e.type === 'conflict').length ?? 0
  const similarCount = data?.edges.filter((e) => e.type === 'similar').length ?? 0

  return (
    <div className="max-w-6xl mx-auto">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Knowledge Graph</h1>
          <p className="text-gray-500">Visualize memory relationships, conflicts, and connections.</p>
        </div>
        <button
          onClick={load}
          className="text-sm px-3 py-1.5 bg-gray-100 hover:bg-gray-200 rounded-lg text-gray-700"
        >
          ↻ Refresh
        </button>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-4 mb-3 text-xs text-gray-600">
        <span className="flex items-center gap-1.5">
          <span className="inline-block w-6 h-0.5" style={{ background: '#dc2626', borderTop: '2px dashed #dc2626' }} />
          Conflict ({conflictCount})
        </span>
        <span className="flex items-center gap-1.5">
          <span className="inline-block w-6 h-0.5" style={{ background: '#9ca3af' }} />
          Similar ({similarCount})
        </span>
        <span className="text-gray-400">{data?.nodes.length ?? 0} memories</span>
      </div>

      {error && (
        <div className="bg-red-50 text-red-700 rounded-lg p-3 mb-3 text-sm">{error}</div>
      )}

      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden" style={{ height: '600px' }}>
        {loading ? (
          <div className="flex items-center justify-center h-full text-gray-400">Loading graph…</div>
        ) : !data || data.nodes.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-gray-400">
            <div className="text-4xl mb-2">🕸️</div>
            <p>No memories yet. Upload or write knowledge to populate the graph.</p>
          </div>
        ) : (
          <ReactFlow
            nodes={nodes}
            edges={styledEdges}
            nodeTypes={nodeTypes}
            onNodeClick={onNodeClick}
            fitView
            fitViewOptions={{ padding: 0.2 }}
            proOptions={{ hideAttribution: true }}
          >
            <Background color="#f1f5f9" gap={16} />
            <Controls />
            <MiniMap
              nodeColor={(n) => {
                const p = (n.data as GraphNodeData)?.provenance
                return p === 'ai' ? '#a855f7' : p === 'co' ? '#3b82f6' : '#10b981'
              }}
              maskColor="rgba(0,0,0,0.05)"
            />
          </ReactFlow>
        )}
      </div>
      <p className="text-xs text-gray-400 mt-2">Tip: click a node to highlight its connections.</p>
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
