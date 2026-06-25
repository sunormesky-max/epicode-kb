import { useState } from 'react'

export default function AgentIntegration() {
  const [spaceId, setSpaceId] = useState('')
  const [name, setName] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [loading, setLoading] = useState(false)

  const createKey = async () => {
    if (!spaceId || !name) return
    setLoading(true)
    try {
      const res = await fetch(`/api/v1/spaces/${spaceId}/api-keys`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: JSON.stringify({ name, scope: 'write' }),
      })
      const data = await res.json()
      if (data.code === 0 && data.data) {
        setApiKey(data.data.key)
      }
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="p-8 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Agent Integration</h1>
      <div className="bg-white rounded-lg shadow p-6 space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700">Space ID</label>
          <input
            value={spaceId}
            onChange={(e) => setSpaceId(e.target.value)}
            className="mt-1 w-full rounded border-gray-300 shadow-sm"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-gray-700">Key Name</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="mt-1 w-full rounded border-gray-300 shadow-sm"
          />
        </div>
        <button
          onClick={createKey}
          disabled={loading}
          className="bg-blue-600 text-white py-2 px-4 rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {loading ? 'Creating...' : 'Create API Key'}
        </button>
        {apiKey && (
          <div className="p-3 bg-green-50 text-green-800 rounded text-sm break-all">
            <strong>API Key:</strong> {apiKey}
          </div>
        )}
      </div>
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
