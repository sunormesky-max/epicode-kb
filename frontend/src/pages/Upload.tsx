import { useState } from 'react'
import UploadZone from '../components/UploadZone'
import { upload, ApiError } from '../lib/api'
import type { UploadResponse } from '../lib/types'

const DEFAULT_SPACE = 'sp_default'

export default function Upload() {
  const [result, setResult] = useState<UploadResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [spaceId, setSpaceId] = useState(DEFAULT_SPACE)
  const [provenance, setProvenance] = useState('human')

  const handleFile = async (file: File) => {
    setLoading(true)
    setError(null)
    setResult(null)
    try {
      const res = await upload(file, spaceId, provenance)
      setResult(res)
    } catch (e) {
      if (e instanceof ApiError) {
        setError(e.message)
      } else {
        setError(String(e))
      }
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold text-gray-900 mb-1">Upload Document</h1>
      <p className="text-gray-500 mb-6">Upload a file to parse and create memories.</p>

      {/* Options */}
      <div className="flex gap-4 mb-6">
        <div className="flex-1">
          <label className="block text-xs font-medium text-gray-600 mb-1">Space ID</label>
          <input
            type="text"
            value={spaceId}
            onChange={(e) => setSpaceId(e.target.value)}
            className="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-lg"
          />
        </div>
        <div className="flex-1">
          <label className="block text-xs font-medium text-gray-600 mb-1">Provenance</label>
          <select
            value={provenance}
            onChange={(e) => setProvenance(e.target.value)}
            className="w-full px-3 py-1.5 text-sm border border-gray-300 rounded-lg bg-white"
          >
            <option value="human">🟢 Human</option>
            <option value="ai">🟣 AI</option>
            <option value="co">🔵 Collaborative</option>
            <option value="conflict">🔴 Conflict</option>
          </select>
        </div>
      </div>

      {/* Upload zone */}
      <UploadZone onFile={handleFile} disabled={loading} />

      {/* Error */}
      {error && (
        <div className="mt-4 p-4 bg-red-50 border border-red-200 rounded-lg">
          <p className="text-sm text-red-700 font-medium">Upload failed</p>
          <p className="text-sm text-red-600 mt-1">{error}</p>
        </div>
      )}

      {/* Result */}
      {result && (
        <div className="mt-6">
          <div className="bg-green-50 border border-green-200 rounded-lg p-4 mb-4">
            <p className="text-sm text-green-700 font-medium">
              ✓ Uploaded {result.file_name} ({result.file_type})
            </p>
            <p className="text-sm text-green-600 mt-1">
              {result.memories_created.length} memories created from {result.total_chunks} chunks
              in {result.processing_time_ms}ms
            </p>
          </div>

          {/* Memory list */}
          <h3 className="text-sm font-semibold text-gray-700 mb-2">Created Memories</h3>
          <div className="space-y-2">
            {result.memories_created.map((mem) => (
              <div
                key={mem.id}
                className="bg-white border border-gray-200 rounded-lg p-3 flex items-start gap-3"
              >
                <span className="text-xs font-mono text-gray-400 mt-0.5">
                  #{mem.chunk_index}
                </span>
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-mono text-blue-600 mb-1">{mem.id}</p>
                  <p className="text-sm text-gray-600 line-clamp-2">{mem.content_preview}</p>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
