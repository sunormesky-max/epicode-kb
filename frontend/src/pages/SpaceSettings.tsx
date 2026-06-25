import { useState } from 'react'
import { useParams } from 'react-router-dom'

export default function SpaceSettings() {
  const { id } = useParams<{ id: string }>()
  const [visibility, setVisibility] = useState('team')
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState('')

  const handleSave = async () => {
    if (!id) return
    setSaving(true)
    try {
      const res = await fetch(`/api/v1/spaces/${id}/visibility`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: JSON.stringify({ visibility }),
      })
      const data = await res.json()
      setMessage(data.code === 0 ? 'Saved successfully.' : data.message || 'Save failed')
    } catch (err) {
      setMessage(String(err))
    } finally {
      setSaving(false)
    }
  }

  return (
    <div className="p-8 max-w-3xl mx-auto">
      <h1 className="text-2xl font-bold mb-6">Space Settings</h1>
      <div className="bg-white rounded-lg shadow p-6 space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700">Visibility</label>
          <select
            value={visibility}
            onChange={(e) => setVisibility(e.target.value)}
            className="mt-1 block w-full rounded border-gray-300 shadow-sm"
          >
            <option value="private">Private</option>
            <option value="team">Team</option>
            <option value="public">Public</option>
          </select>
        </div>
        {message && (
          <div className="p-3 bg-blue-50 text-blue-700 rounded text-sm">{message}</div>
        )}
        <button
          onClick={handleSave}
          disabled={saving}
          className="bg-blue-600 text-white py-2 px-4 rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
