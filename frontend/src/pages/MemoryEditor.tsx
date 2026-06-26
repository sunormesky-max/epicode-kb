import { useEffect, useState } from 'react'
import { useParams } from 'react-router-dom'
import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import Collaboration from '@tiptap/extension-collaboration'
import CollaborationCursor from '@tiptap/extension-collaboration-cursor'
import * as Y from 'yjs'
import { WebsocketProvider } from 'y-websocket'
import SidePanel from '../components/SidePanel'

const CURSOR_COLORS = ['#f58231', '#911eb4', '#46f0f0', '#f032e6', '#bcf60c', '#fabed4', '#ffe119', '#4363d8']

function pickColor(seed: string): string {
  let h = 0
  for (let i = 0; i < seed.length; i++) h = (h * 31 + seed.charCodeAt(i)) >>> 0
  return CURSOR_COLORS[h % CURSOR_COLORS.length]
}

interface CollabState {
  ydoc: Y.Doc
  provider: WebsocketProvider
}

export default function MemoryEditor() {
  const { id } = useParams<{ id: string }>()
  const [collab, setCollab] = useState<CollabState | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)

  // Load existing memory, then spin up a Yjs doc + y-websocket provider.
  useEffect(() => {
    if (!id) return
    let doc: Y.Doc | null = null
    let provider: WebsocketProvider | null = null
    let cancelled = false

    fetch(`/api/v1/memories/${id}`, { headers: authHeaders() })
      .then((r) => r.json())
      .then((data) => {
        if (cancelled) return
        if (data.code !== 0 || !data.data) {
          setLoading(false)
          return
        }
        const content: string = data.data.content || ''
        doc = new Y.Doc()
        // Seed the doc with existing content (plain text). y-websocket sync
        // will reconcile with the server-side yrs doc afterwards.
        if (content) {
          doc.getText('content').insert(0, content)
        }
        const wsProtocol = location.protocol === 'https:' ? 'wss' : 'ws'
        const token = localStorage.getItem('access_token') || ''
        // y-websocket connects to `${wsUrl}/${roomname}`. We point wsUrl at the
        // collab base, use the memory id as roomname, and pass the JWT via params.
        provider = new WebsocketProvider(
          `${wsProtocol}://${location.host}/api/v1/collab`,
          id,
          doc,
          { params: { token }, disableBc: true },
        )
        const user = {
          name: localStorage.getItem('user_name') || 'Anonymous',
          color: pickColor(localStorage.getItem('user_id') || id),
        }
        provider.awareness.setLocalStateField('user', user)
        setCollab({ ydoc: doc, provider })
        setLoading(false)
      })
      .catch(() => setLoading(false))

    return () => {
      cancelled = true
      provider?.destroy()
      doc?.destroy()
    }
  }, [id])

  const editor = useEditor(
    {
      extensions: [
        // History is handled by Yjs (undo/redo), so disable the built-in.
        StarterKit.configure({ history: false }),
        ...(collab
          ? [Collaboration.configure({ document: collab.ydoc, field: 'content' })]
          : []),
        ...(collab
          ? [
              CollaborationCursor.configure({
                provider: collab.provider,
                user: {
                  name: localStorage.getItem('user_name') || 'Anonymous',
                  color: pickColor(localStorage.getItem('user_id') || id || 'anon'),
                },
              }),
            ]
          : []),
      ],
      editorProps: {
        attributes: {
          class: 'prose max-w-none min-h-[400px] p-4 focus:outline-none',
        },
      },
    },
    [collab],
  )

  const handleSave = async () => {
    if (!editor || !id) return
    setSaving(true)
    try {
      const html = editor.getHTML()
      await fetch(`/api/v1/memories/${id}/save`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...authHeaders() },
        body: JSON.stringify({ content: html }),
      })
    } finally {
      setSaving(false)
    }
  }

  if (loading || !editor) {
    return <div className="p-8 text-gray-500">Loading editor…</div>
  }

  return (
    <div className="flex gap-4 h-full">
      <div className="flex-1 p-8 max-w-4xl mx-auto">
        <div className="flex items-center justify-between mb-4">
          <h1 className="text-2xl font-bold text-gray-900">Edit Memory</h1>
          <div className="flex items-center gap-3">
            {collab && (
              <span className="text-xs text-gray-400">
                {collab.provider.awareness.getStates().size} online
              </span>
            )}
            <button
              onClick={handleSave}
              disabled={saving}
              className="bg-blue-600 text-white py-2 px-4 rounded hover:bg-blue-700 disabled:opacity-50"
            >
              {saving ? 'Saving…' : 'Save Version'}
            </button>
          </div>
        </div>
        <div className="bg-white rounded-lg shadow border border-gray-200">
          <EditorContent editor={editor} />
        </div>
        <p className="text-xs text-gray-400 mt-2">
          Real-time collaboration active. Edits sync to others live; press Save to persist a version.
        </p>
      </div>
      {id && <SidePanel memoryId={id} editorContent={editor.getText()} />}
    </div>
  )
}

function authHeaders(): Record<string, string> {
  const token = localStorage.getItem('access_token')
  return token ? { Authorization: `Bearer ${token}` } : {}
}
