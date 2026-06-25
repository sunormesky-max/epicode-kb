import type { Provenance } from '../lib/types'

const config: Record<Provenance, { label: string; emoji: string; className: string }> = {
  human: { label: 'Human', emoji: '🟢', className: 'bg-green-100 text-green-800' },
  ai: { label: 'AI', emoji: '🟣', className: 'bg-purple-100 text-purple-800' },
  co: { label: 'Collab', emoji: '🔵', className: 'bg-teal-100 text-teal-800' },
  conflict: { label: 'Conflict', emoji: '🔴', className: 'bg-red-100 text-red-800' },
}

export default function ProvenanceBadge({ provenance }: { provenance: Provenance }) {
  const c = config[provenance] ?? config.human
  return (
    <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${c.className}`}>
      <span>{c.emoji}</span>
      {c.label}
    </span>
  )
}
