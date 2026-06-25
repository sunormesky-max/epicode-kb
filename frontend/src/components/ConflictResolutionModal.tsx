interface ConflictResolutionModalProps {
  open: boolean
  onClose: () => void
}

export default function ConflictResolutionModal({ open, onClose }: ConflictResolutionModalProps) {
  if (!open) return null
  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-white rounded-lg shadow p-6 max-w-lg w-full">
        <h2 className="text-lg font-bold mb-4">Resolve Conflict</h2>
        <p className="text-gray-600 mb-4">Conflict resolution UI placeholder.</p>
        <div className="flex justify-end space-x-2">
          <button onClick={onClose} className="px-4 py-2 border rounded hover:bg-gray-50">
            Cancel
          </button>
          <button onClick={onClose} className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
            Resolve
          </button>
        </div>
      </div>
    </div>
  )
}
