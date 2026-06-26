import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import SidePanel from '../components/SidePanel'

interface Memory {
  id: string
  content: string
  space_id: string
}

export default function MemoryEditor() {
  const { id } = useParams<{ id: string }>()
  const [memory, setMemory] = useState<Memory | null>(null)
  const [content, setContent] = useState('')
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (!id) return
    fetch(`/api/v1/memories/${id}`, { headers: authHeaders() })
      .then((res) => res.json())
      .then((data) => {
        if (data.code === 0 && data.data) {
          setMemory(data.data)
          setContent(data.data.content)
        }
      })
  }, [id])

  const handleSave = async () => {
    if (!id) return
    setSaving(true)
    try {
      await fetch(`/api/v1/memories/${id}/save`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: JSON.stringify({ content }),
      })
    } finally {
      setSaving(false)
    }
  }

  if (!memory) {
    return <div className="p-8 text-gray-500">Loading...</div>
  }

  return (
    <div className="flex gap-4 h-full">
      <div className="flex-1 p-8 max-w-4xl mx-auto">
        <h1 className="text-2xl font-bold mb-6">Edit Memory</h1>
        <div className="bg-white rounded-lg shadow p-6 space-y-4">
          <textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            rows={16}
            className="w-full rounded border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 font-mono"
          />
          <div className="flex justify-end">
            <button
              onClick={handleSave}
              disabled={saving}
              className="bg-blue-600 text-white py-2 px-4 rounded hover:bg-blue-700 disabled:opacity-50"
            >
              {saving ? 'Saving...' : 'Save Version'}
            </button>
          </div>
        </div>
      </div>
      {id && <SidePanel memoryId={id} editorContent={content} />}
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
