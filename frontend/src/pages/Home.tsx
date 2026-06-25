import { useState, useEffect } from 'react'
import { Link } from 'react-router-dom'
import { systemHealth } from '../lib/api'

export default function Home() {
  const [health, setHealth] = useState<{ status: string; version: string } | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    systemHealth()
      .then(setHealth)
      .catch((e) => setError(e.message))
  }, [])

  return (
    <div className="max-w-4xl mx-auto">
      <h1 className="text-3xl font-bold text-gray-900 mb-2">🧠 epicode-kb</h1>
      <p className="text-gray-600 mb-8">
        Enterprise knowledge base with memory provenance, hybrid search, and AI proposal engine.
      </p>

      {/* System status */}
      <div className="bg-white rounded-lg border border-gray-200 p-4 mb-8">
        <h2 className="text-sm font-semibold text-gray-700 mb-2">System Status</h2>
        {error ? (
          <p className="text-sm text-red-600">⚠️ Backend not reachable: {error}</p>
        ) : health ? (
          <div className="flex items-center gap-4 text-sm">
            <span className="inline-flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-green-500" />
              <span className="font-medium text-green-700">{health.status}</span>
            </span>
            <span className="text-gray-500">v{health.version}</span>
          </div>
        ) : (
          <p className="text-sm text-gray-400">Checking...</p>
        )}
      </div>

      {/* Quick links */}
      <h2 className="text-lg font-semibold text-gray-800 mb-3">Quick Actions</h2>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Link
          to="/upload"
          className="bg-white rounded-lg border border-gray-200 p-6 hover:border-blue-400 hover:shadow-md transition-all"
        >
          <div className="text-3xl mb-2">📄</div>
          <h3 className="font-semibold text-gray-900 mb-1">Upload Document</h3>
          <p className="text-sm text-gray-500">Upload markdown, text, or PDF files to create memories.</p>
        </Link>

        <Link
          to="/search"
          className="bg-white rounded-lg border border-gray-200 p-6 hover:border-blue-400 hover:shadow-md transition-all"
        >
          <div className="text-3xl mb-2">🔍</div>
          <h3 className="font-semibold text-gray-900 mb-1">Search Memories</h3>
          <p className="text-sm text-gray-500">Hybrid search across semantic and full-text indexes.</p>
        </Link>

        <Link
          to="/review"
          className="bg-white rounded-lg border border-gray-200 p-6 hover:border-blue-400 hover:shadow-md transition-all"
        >
          <div className="text-3xl mb-2">📋</div>
          <h3 className="font-semibold text-gray-900 mb-1">Review Queue</h3>
          <p className="text-sm text-gray-500">Review and approve AI-proposed memories.</p>
        </Link>
      </div>

      {/* Feature overview */}
      <h2 className="text-lg font-semibold text-gray-800 mt-8 mb-3">Features</h2>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <h3 className="font-medium text-gray-900 mb-1">Memory Provenance</h3>
          <p className="text-sm text-gray-500">
            Every memory tracks its source (human, AI, collaborative, conflict) with trust levels and review status.
          </p>
        </div>
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <h3 className="font-medium text-gray-900 mb-1">Hybrid Search</h3>
          <p className="text-sm text-gray-500">
            Combines semantic vector similarity with Tantivy full-text search using RRF fusion and trust weighting.
          </p>
        </div>
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <h3 className="font-medium text-gray-900 mb-1">AI Proposal Engine</h3>
          <p className="text-sm text-gray-500">
            AI detects duplicates, clusters, and contradictions, generating merge/link/summarize proposals for review.
          </p>
        </div>
        <div className="bg-white rounded-lg border border-gray-200 p-4">
          <h3 className="font-medium text-gray-900 mb-1">Knowledge Health</h3>
          <p className="text-sm text-gray-500">
            Monitors knowledge gaps, stale memories, orphan memories, and conflict counts per space.
          </p>
        </div>
      </div>
    </div>
  )
}
